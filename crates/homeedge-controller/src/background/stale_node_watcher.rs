use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use tokio::time::{interval, MissedTickBehavior};

use crate::{app_state::AppState, background::reassignment_loop::reassign_from_offline_nodes};
use homeedge_types::{NodeId, NodeRecord, NodeStatus};


fn stale_node_ids(
    nodes: &HashMap<NodeId, NodeRecord>,
    now: DateTime<Utc>,
    heartbeat_timeout: Duration,
) -> Vec<NodeId> {
    let mut stale = Vec::new();

    for node in nodes.values() {
        let is_stale = match node.last_heartbeat {
            Some(last_heartbeat) => {
                let elapsed = now
                    .signed_duration_since(last_heartbeat)
                    .to_std()
                    .unwrap_or(Duration::ZERO);

                elapsed > heartbeat_timeout
            }
            None => false,
        };

        if is_stale && node.status != NodeStatus::Offline {
            stale.push(node.id);
        }
    }

    stale
}


pub async fn run_stale_node_watcher(
    state: AppState,
    check_interval: Duration,
    heartbeat_timeout: Duration,
) {
    let mut ticker = interval(check_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let now = Utc::now();

        let stale_ids = {
            let guard = state.inner.lock().await;
            stale_node_ids(&guard.nodes, now, heartbeat_timeout)
        };

        if stale_ids.is_empty() {
            continue;
        }

        for node_id in stale_ids {
            let previous_status = {
                let guard = state.inner.lock().await;
                guard.nodes.get(&node_id).map(|n| n.status)
            };

            if let Some(previous_status) = previous_status {
                if let Err(err) = state.mark_node_offline(node_id).await {
                    tracing::error!(
                        node_id = %node_id,
                        error = %err,
                        "failed to persist offline node transition"
                    );
                    continue;
                }

                tracing::warn!(
                    node_id = %node_id,
                    from = ?previous_status,
                    to = ?NodeStatus::Offline,
                    "node status changed"
                );
            }
        }

        if let Err(err) = state.reassign_from_offline_nodes().await {
            tracing::error!(
                error = %err,
                "failed to persist reassignment after offline detection"
            );
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use chrono::{Duration as ChronoDuration, Utc};
    use uuid::Uuid;

    use homeedge_types::{
        node::{NodeId, NodeRecord},
        NodeStatus,
    };

    use super::stale_node_ids;

    fn node_id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    #[test]
    fn stale_node_ids_returns_old_heartbeat_node() {
        let now = Utc::now();
        let id = node_id(1);

        let nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Healthy,
                last_heartbeat: Some(now - ChronoDuration::seconds(31)),
                capabilities: vec!["docker".into()],
            },
        )]);

        let stale = stale_node_ids(&nodes, now, Duration::from_secs(30));

        assert_eq!(stale, vec![id]);
    }

    #[test]
    fn stale_node_ids_ignores_recent_heartbeat() {
        let now = Utc::now();
        let id = node_id(2);

        let nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Healthy,
                last_heartbeat: Some(now - ChronoDuration::seconds(5)),
                capabilities: vec!["docker".into()],
            },
        )]);

        let stale = stale_node_ids(&nodes, now, Duration::from_secs(30));

        assert!(stale.is_empty());
    }

    #[test]
    fn stale_node_ids_ignores_already_offline_node() {
        let now = Utc::now();
        let id = node_id(3);

        let nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Offline,
                last_heartbeat: Some(now - ChronoDuration::seconds(60)),
                capabilities: vec![],
            },
        )]);

        let stale = stale_node_ids(&nodes, now, Duration::from_secs(30));

        assert!(stale.is_empty());
    }
}
