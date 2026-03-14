use chrono::{DateTime, Utc};
use homeedge_types::node::{NodeRecord, NodeStatus};


pub fn on_register(existing: Option<&NodeRecord>, incoming: NodeRecord) -> NodeRecord {
    match existing {
        None => NodeRecord {
            status: NodeStatus::Registering,
            ..incoming
        },
        Some(current) => {
            let mut updated = incoming;
            updated.status = NodeStatus::Registering;
            updated.last_heartbeat = current.last_heartbeat;
            updated
        }
    }
}

pub fn on_heartbeat(node: &mut NodeRecord, timestamp: DateTime<Utc>) {
    node.last_heartbeat = Some(timestamp);
    node.status = NodeStatus::Healthy;
}

pub fn mark_offline(node: &mut NodeRecord) {
    node.status = NodeStatus::Offline;
}


#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use homeedge_types::node::{NodeId, NodeRecord, NodeStatus};

    use super::*;


    fn sample_node(status: NodeStatus) -> NodeRecord {
        NodeRecord {
            id: NodeId(Uuid::new_v4()),
            status,
            last_heartbeat: None,
            capabilities: vec!["docker".into()],
        }
    }

    #[test]
    fn register_unknown_node_sets_registering() {
        let incoming = sample_node(NodeStatus::Healthy);

        let updated = on_register(None, incoming);

        assert_eq!(updated.status, NodeStatus::Registering);
        assert!(updated.last_heartbeat.is_none());
    }

    #[test]
    fn register_existing_node_resets_to_registering_and_preserves_last_heartbeat() {
        let mut existing = sample_node(NodeStatus::Healthy);
        let ts = Utc::now();
        existing.last_heartbeat = Some(ts);

        let mut incoming = sample_node(NodeStatus::Offline);
        incoming.id = existing.id;

        let updated = on_register(Some(&existing), incoming);

        assert_eq!(updated.status, NodeStatus::Registering);
        assert_eq!(updated.last_heartbeat, Some(ts));
    }

    #[test]
    fn heartbeat_moves_registering_to_healthy() {
        let ts = Utc::now();
        let mut node = sample_node(NodeStatus::Registering);

        on_heartbeat(&mut node, ts);

        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.last_heartbeat, Some(ts));
    }

    #[test]
    fn heartbeat_keeps_healthy_as_healthy_and_updates_timestamp() {
        let ts = Utc::now();
        let mut node = sample_node(NodeStatus::Healthy);

        on_heartbeat(&mut node, ts);

        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.last_heartbeat, Some(ts));
    }

    #[test]
    fn heartbeat_recovers_offline_node_to_healthy() {
        let ts = Utc::now();
        let mut node = sample_node(NodeStatus::Offline);
        on_heartbeat(&mut node, ts);
        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.last_heartbeat, Some(ts));
    }

    #[test]
    fn mark_offline_sets_status_offline() {
        let mut node = sample_node(NodeStatus::Healthy);

        mark_offline(&mut node);

        assert_eq!(node.status, NodeStatus::Offline);
    }
}
