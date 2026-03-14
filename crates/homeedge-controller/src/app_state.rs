use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use homeedge_types::{HeartbeatRequest, NodeStatus, RegisterRequest, ServiceAssignment, ServiceDefinition};
use homeedge_types::node::{NodeId, NodeRecord};
use homeedge_types::service::ServiceId;

use crate::domain::node_registry::{on_heartbeat, on_register};
use crate::error::AppError;

#[derive(Debug, Default)]
pub struct ControllerState {
    pub nodes: HashMap<NodeId, NodeRecord>,
    pub services: HashMap<ServiceId, ServiceDefinition>, // later: ServiceDefinition
    pub assignments: HashMap<NodeId, Vec<ServiceId>>,
}

impl ControllerState {
    pub fn register_node(&mut self, req: RegisterRequest) -> NodeRecord {
        let incoming = NodeRecord {
            id: req.node_id,
            status: NodeStatus::Registering,
            last_heartbeat: None,
            capabilities: req.capabilities,
        };

        let updated = on_register(self.nodes.get(&incoming.id), incoming);

        self.assignments.entry(updated.id).or_default();
        self.nodes.insert(updated.id, updated.clone());

        updated
    }

    pub fn record_heartbeat(&mut self, req: HeartbeatRequest) -> Result<NodeRecord, AppError> {
        let node = self
            .nodes
            .get_mut(&req.node_id)
            .ok_or(AppError::NodeNotFound)?;

        on_heartbeat(node, req.timestamp);
        Ok(node.clone())
    }

    pub fn assignments_for(&self, node_id: NodeId) -> Result<Vec<ServiceAssignment>, AppError> {
        if !self.nodes.contains_key(&node_id) {
            return Err(AppError::NodeNotFound);
        }

        let assignments = self.assignments
            .get(&node_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|service_id| self.services.contains_key(service_id))
            .map(|service_id| ServiceAssignment { service_id, node_id })
            .collect();

        Ok(assignments)
    }
        pub fn list_nodes(&self) -> Vec<NodeRecord> {
            let mut nodes: Vec<_> = self.nodes.values().cloned().collect();
            nodes.sort_by_key(|n| n.id.0);
            nodes
        }
    }

pub type SharedState = Arc<Mutex<ControllerState>>;

#[derive(Clone)]
pub struct AppState {
    pub inner: SharedState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ControllerState::default())),
        }
    }
}


#[cfg(test)]
mod tests {
    use chrono::Utc;
    use homeedge_types::{ServiceAssignment, ServiceDefinition};
    use uuid::Uuid;

    use homeedge_types::api::{HeartbeatRequest, RegisterRequest};
    use homeedge_types::node::{NodeId, NodeStatus};
    use homeedge_types::service::ServiceId;

    use super::ControllerState;

    fn node_id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    fn service_id(n: u128) -> ServiceId {
        ServiceId(Uuid::from_u128(n))
    }

    #[test]
    fn register_node_creates_new_registering_node() {
        let mut state = ControllerState::default();
        let id = node_id(1);

        let node = state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec!["docker".into(), "mqtt".into()],
        });

        assert_eq!(node.id, id);
        assert_eq!(node.status, NodeStatus::Registering);
        assert_eq!(node.last_heartbeat, None);
        assert_eq!(node.capabilities, vec!["docker", "mqtt"]);

        let stored = state.nodes.get(&id).unwrap();
        assert_eq!(stored.status, NodeStatus::Registering);

        let assignments = state.assignments.get(&id).unwrap();
        assert!(assignments.is_empty());
    }

    #[test]
    fn register_node_updates_existing_node_capabilities() {
        let mut state = ControllerState::default();
        let id = node_id(2);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec!["docker".into()],
        });

        let node = state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec!["zigbee".into(), "ble".into()],
        });

        assert_eq!(node.id, id);
        assert_eq!(node.status, NodeStatus::Registering);
        assert_eq!(node.capabilities, vec!["zigbee", "ble"]);

        let stored = state.nodes.get(&id).unwrap();
        assert_eq!(stored.capabilities, vec!["zigbee", "ble"]);
    }

    #[test]
    fn record_heartbeat_marks_node_healthy() {
        let mut state = ControllerState::default();
        let id = node_id(3);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec!["docker".into()],
        });

        let ts = Utc::now();

        let node = state
            .record_heartbeat(HeartbeatRequest {
                node_id: id,
                timestamp: ts,
                service_statuses: vec![],
            })
            .unwrap();

        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.last_heartbeat, Some(ts));

        let stored = state.nodes.get(&id).unwrap();
        assert_eq!(stored.status, NodeStatus::Healthy);
        assert_eq!(stored.last_heartbeat, Some(ts));
    }

    #[test]
    fn record_heartbeat_fails_for_unknown_node() {
        let mut state = ControllerState::default();

        let result = state.record_heartbeat(HeartbeatRequest {
            node_id: node_id(999),
            timestamp: Utc::now(),
            service_statuses: vec![],
        });

        assert!(result.is_err());
    }

    #[test]
    fn assignments_for_known_node_without_assignments_returns_empty_vec() {
        let mut state = ControllerState::default();
        let id = node_id(4);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec![],
        });

        let assignments = state.assignments_for(id).unwrap();
        assert!(assignments.is_empty());
    }

    #[test]
    fn assignments_for_unknown_node_returns_error() {
        let state = ControllerState::default();

        let result = state.assignments_for(node_id(404));
        assert!(result.is_err());
    }

    #[test]
    fn list_nodes_returns_sorted_nodes() {
        let mut state = ControllerState::default();

        let id_b = node_id(20);
        let id_a = node_id(10);

        state.register_node(RegisterRequest {
            node_id: id_b,
            capabilities: vec!["b".into()],
        });

        state.register_node(RegisterRequest {
            node_id: id_a,
            capabilities: vec!["a".into()],
        });

        let nodes = state.list_nodes();

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, id_a);
        assert_eq!(nodes[1].id, id_b);
    }

    #[test]
    fn assignments_for_returns_existing_assignments() {
        let mut state = ControllerState::default();
        let id = node_id(5);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec![],
        });

        let s1 = service_id(100);
        let s2 = service_id(200);

        state.services.insert(s1, ServiceDefinition::new("svc1", "v1"));
        state.services.insert(s2, ServiceDefinition::new("svc2", "v1"));
        state.assignments.insert(id, vec![s1, s2]);

        let assignments = state.assignments_for(id).unwrap();
        assert_eq!(
            assignments,
            vec![
                ServiceAssignment {
                    service_id: s1,
                    node_id: id,
                },
                ServiceAssignment {
                    service_id: s2,
                    node_id: id,
                },
            ]
        );
    }

    #[test]
    fn assignments_for_filters_out_stale_service_ids() {
        let mut state = ControllerState::default();
        let id = node_id(6);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec![],
        });

        let valid = service_id(100);
        let stale = service_id(200);

        state.services.insert(valid, ServiceDefinition::new("svc1", "v1"));
        // stale is intentionally not inserted into services
        state.assignments.insert(id, vec![valid, stale]);

        let assignments = state.assignments_for(id).unwrap();
        assert_eq!(
            assignments,
            vec![ServiceAssignment {
                service_id: valid,
                node_id: id,
            }]
        );
    }
}
