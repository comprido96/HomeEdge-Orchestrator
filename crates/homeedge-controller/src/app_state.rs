use std::collections::HashMap;
use std::sync::Arc;

use homeedge_types::api::NodeView;
use tokio::sync::Mutex;

use homeedge_types::{HeartbeatRequest, NodeStatus, RegisterRequest, ServiceAssignment, ServiceDefinition, ServiceStatus};
use homeedge_types::node::{NodeId, NodeRecord};
use homeedge_types::service::{ServiceHealthReport, ServiceId};

use crate::domain::node_registry::{on_heartbeat, on_register};
use crate::error::AppError;

#[derive(Debug)]
pub struct ControllerState {
    pub nodes: HashMap<NodeId, NodeRecord>,
    pub services: HashMap<ServiceId, ServiceDefinition>, // later: ServiceDefinition
    pub assignments: HashMap<NodeId, Vec<ServiceId>>,
    pub observed: HashMap<NodeId, HashMap<ServiceId, ServiceStatus>>,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            services: HashMap::new(),
            assignments: HashMap::new(),
            observed: HashMap::new(),
        }
    }
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

        let observed = req.service_statuses
            .into_iter()
            .map(|r| (r.service_id, r.status))
            .collect();

        self.observed.insert(req.node_id, observed);

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

    pub fn list_assignments(&self) -> Vec<ServiceAssignment> {
        let mut result = Vec::new();

        for (node_id, services) in &self.assignments {
            for service_id in services {
                if self.services.contains_key(service_id) {
                    result.push(ServiceAssignment {
                        service_id: *service_id,
                        node_id: *node_id,
                    });
                }
            }
        }

        result.sort_by_key(|a| (a.node_id.0, a.service_id.0));

        result
    }

    pub fn list_nodes(&self) -> Vec<NodeRecord> {
        let mut nodes: Vec<_> = self.nodes.values().cloned().collect();
        nodes.sort_by_key(|n| n.id.0);
        nodes
    }

    pub fn assign_service(
        &mut self,
        service_id: ServiceId,
        node_id: NodeId,
    ) -> Result<ServiceAssignment, AppError> {

        if !self.services.contains_key(&service_id) {
            return Err(AppError::ServiceNotFound);
        }

        if !self.nodes.contains_key(&node_id) {
            return Err(AppError::NodeNotFound);
        }

        // remove from all nodes first
        for services in self.assignments.values_mut() {
            services.retain(|id| *id != service_id);
        }

        let node_services = self.assignments.entry(node_id).or_default();

        if !node_services.contains(&service_id) {
            node_services.push(service_id);
        }

        tracing::info!(
            service_id = %service_id,
            node_id = %node_id,
            "service assigned"
        );

        Ok(ServiceAssignment {
            service_id,
            node_id,
        })
    }

    pub fn unassign_service(
        &mut self,
        service_id: ServiceId,
    ) -> Result<(), AppError> {

        for services in self.assignments.values_mut() {
            services.retain(|id| *id != service_id);
        }

        tracing::info!(
            service_id = %service_id,
            "service unassigned"
        );

        Ok(())
    }

    pub fn delete_service(
        &mut self,
        service_id: ServiceId,
    ) -> Result<(), AppError> {

        if !self.services.contains_key(&service_id) {
            return Err(AppError::ServiceNotFound);
        }

        self.services.remove(&service_id);

        // cleanup assignments
        for services in self.assignments.values_mut() {
            services.retain(|id| *id != service_id);
        }

        tracing::info!(
            service_id = %service_id,
            "service deleted"
        );

        Ok(())
    }

    pub fn get_service(
        &self,
        service_id: ServiceId,
    ) -> Result<ServiceDefinition, AppError> {

        self.services
            .get(&service_id)
            .cloned()
            .ok_or(AppError::ServiceNotFound)
    }

    pub fn update_service(
        &mut self,
        service_id: ServiceId,
        name: String,
        version: String,
        selector: Option<String>,
    ) -> Result<ServiceDefinition, AppError> {

        // Validate inputs first — no borrows yet
        if name.trim().is_empty() {
            return Err(AppError::BadRequest("name must not be empty".into()));
        }

        if version.trim().is_empty() {
            return Err(AppError::BadRequest("version must not be empty".into()));
        }

        // Conflict check before mutable borrow
        if self.services.values().any(|s|
            s.id != service_id &&
            s.name == name &&
            s.version == version
        ) {
            return Err(AppError::Conflict(
                format!("service '{}' version '{}' already exists", name, version)
            ));
        }

        // Check existence before mutable borrow too
        if !self.services.contains_key(&service_id) {
            return Err(AppError::ServiceNotFound);
        }

        // Now take the mutable borrow — nothing else borrows `self.services` below
        let service = self.services
            .get_mut(&service_id)
            .ok_or(AppError::ServiceNotFound)?;

        service.name = name;
        service.version = version;
        service.selector = selector;

        tracing::info!(
            service_id = %service_id,
            "service updated"
        );

        Ok(service.clone())
    }

    pub fn observed_services(
        &self,
        node_id: NodeId,
    ) -> Vec<ServiceHealthReport> {

        self.observed
            .get(&node_id)
            .map(|m|
                m.iter()
                .map(|(id,status)| ServiceHealthReport{
                    service_id:*id,
                    status:*status
                })
                .collect()
            )
            .unwrap_or_default()
    }

    pub fn list_node_views(&self) -> Vec<NodeView> {
        let mut nodes: Vec<_> = self.nodes.values().cloned().collect();
        nodes.sort_by_key(|n| n.id.0);

        nodes.into_iter()
            .map(|node| {
                let mut services: Vec<ServiceHealthReport> = self.observed
                    .get(&node.id)
                    .map(|m| {
                        let mut reports: Vec<ServiceHealthReport> = m.iter()
                            .map(|(service_id, status)| ServiceHealthReport {
                                service_id: *service_id,
                                status: *status,
                            })
                            .collect();

                        reports.sort_by_key(|r| r.service_id.0);
                        reports
                    })
                    .unwrap_or_default();

                NodeView {
                    node,
                    services,
                }
            })
            .collect()
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

    #[test]
    fn record_heartbeat_updates_timestamp_on_previously_offline_node_with_stale_time() {
        let mut state = ControllerState::default();
        let id = node_id(43);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec![],
        });

        let stale_ts = Utc::now() - chrono::Duration::seconds(120);
        {
            let node = state.nodes.get_mut(&id).unwrap();
            node.status = NodeStatus::Offline;
            node.last_heartbeat = Some(stale_ts);
        }

        let fresh_ts = Utc::now();
        let node = state.record_heartbeat(HeartbeatRequest {
            node_id: id,
            timestamp: fresh_ts,
            service_statuses: vec![],
        }).unwrap();

        assert_eq!(node.status, NodeStatus::Healthy);
        assert_eq!(node.last_heartbeat, Some(fresh_ts));
        assert_ne!(node.last_heartbeat, Some(stale_ts));
    }

    #[test]
    fn record_heartbeat_recovers_offline_node_and_updates_timestamp() {
        let mut state = ControllerState::default();
        let id = node_id(42);

        state.register_node(RegisterRequest {
            node_id: id,
            capabilities: vec!["docker".into()],
        });

        {
            let node = state.nodes.get_mut(&id).unwrap();
            node.status = NodeStatus::Offline;
            node.last_heartbeat = None;
        }

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
    }
}
