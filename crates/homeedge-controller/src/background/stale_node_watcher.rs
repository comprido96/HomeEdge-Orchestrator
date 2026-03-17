use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use tokio::time::{interval, MissedTickBehavior};

use crate::app_state::AppState;
use homeedge_types::{NodeId, NodeRecord, NodeStatus};


fn mark_stale_nodes(
    nodes: &mut HashMap<NodeId, NodeRecord>,
    now: DateTime<Utc>,
    heartbeat_timeout: Duration,
) {
    for node in nodes.values_mut() {
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
            let previous_status = node.status;
            node.status = NodeStatus::Offline;

            tracing::warn!(
                node_id = %node.id,
                from = ?previous_status,
                to = ?NodeStatus::Offline,
                "node status changed"
            );
        }
    }
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
        let mut guard = state.inner.lock().await;

        mark_stale_nodes(&mut guard.nodes, now, heartbeat_timeout);
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use chrono::{Duration as ChronoDuration, Utc};
    use uuid::Uuid;

    use homeedge_types::{
        NodeStatus,
        node::{NodeId, NodeRecord},
    };

    use super::mark_stale_nodes;

    fn node_id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    #[test]
    fn mark_stale_nodes_marks_old_heartbeat_offline() {
        let now = Utc::now();
        let id = node_id(1);

        let mut nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Healthy,
                last_heartbeat: Some(now - ChronoDuration::seconds(31)),
                capabilities: vec!["docker".into()],
            },
        )]);

        mark_stale_nodes(&mut nodes, now, Duration::from_secs(30));

        assert_eq!(nodes.get(&id).unwrap().status, NodeStatus::Offline);
    }

    #[test]
    fn mark_stale_nodes_keeps_recent_heartbeat_healthy() {
        let now = Utc::now();
        let id = node_id(2);

        let mut nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Healthy,
                last_heartbeat: Some(now - ChronoDuration::seconds(5)),
                capabilities: vec!["docker".into()],
            },
        )]);

        mark_stale_nodes(&mut nodes, now, Duration::from_secs(30));

        assert_eq!(nodes.get(&id).unwrap().status, NodeStatus::Healthy);
    }

    #[test]
    fn mark_stale_nodes_does_not_change_already_offline_node() {
        let now = Utc::now();
        let id = node_id(3);

        let mut nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Offline,
                last_heartbeat: Some(now - ChronoDuration::seconds(60)),
                capabilities: vec![],
            },
        )]);

        mark_stale_nodes(&mut nodes, now, Duration::from_secs(30));
        assert_eq!(nodes.get(&id).unwrap().status, NodeStatus::Offline);
    }

    #[test]
    fn mark_stale_nodes_marks_healthy_node_offline_when_heartbeat_is_old() {
        let now = Utc::now();
        let id = node_id(1);

        let mut nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Healthy,
                last_heartbeat: Some(now - chrono::Duration::seconds(31)),
                capabilities: vec!["docker".into()],
            },
        )]);

        mark_stale_nodes(&mut nodes, now, Duration::from_secs(30));

        assert_eq!(nodes.get(&id).unwrap().status, NodeStatus::Offline);
    }

    #[test]
    fn mark_stale_nodes_leaves_offline_node_offline() {
        let now = Utc::now();
        let id = node_id(2);

        let mut nodes = HashMap::from([(
            id,
            NodeRecord {
                id,
                status: NodeStatus::Offline,
                last_heartbeat: Some(now - chrono::Duration::seconds(31)),
                capabilities: vec!["docker".into()],
            },
        )]);

        mark_stale_nodes(&mut nodes, now, Duration::from_secs(30));

        assert_eq!(nodes.get(&id).unwrap().status, NodeStatus::Offline);
    }
}
