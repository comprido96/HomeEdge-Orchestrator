use std::collections::HashMap;
use std::sync::Arc;

use homeedge_types::api::{NodeView, UpdateServiceRequest};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use homeedge_types::{CreateServiceRequest, HeartbeatRequest, NodeStatus, RegisterRequest, ServiceAssignment, ServiceDefinition, ServiceStatus};
use homeedge_types::node::{NodeId, NodeRecord};
use homeedge_types::service::{ServiceHealthReport, ServiceId};

use crate::domain::node_registry::{on_heartbeat, on_register};
use crate::error::AppError;
use crate::repository::{AssignmentRepository, NodeRepository, ObservedStatusRepository, ServiceRepository, SqliteAssignmentRepository, SqliteNodeRepository, SqliteObservedStatusRepository, SqliteServiceRepository};
use crate::background::reassignment_loop::reassign_from_offline_nodes;


#[derive(Clone)]
pub struct SqliteStores {
    pub pool: SqlitePool,
    pub node_repo: SqliteNodeRepository,
    pub service_repo: SqliteServiceRepository,
    pub assignment_repo: SqliteAssignmentRepository,
    pub observed_status_repo: SqliteObservedStatusRepository,
}

impl SqliteStores {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            node_repo: SqliteNodeRepository::new(pool.clone()),
            service_repo: SqliteServiceRepository::new(pool.clone()),
            assignment_repo: SqliteAssignmentRepository::new(pool.clone()),
            observed_status_repo: SqliteObservedStatusRepository::new(pool.clone()),
            pool,
        }
    }
}


#[derive(Clone)]
pub enum StorageMode {
    InMemory,
    Sqlite(SqliteStores),
}


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

        tracing::info!(
            node_id = %updated.id,
            capabilities = ?updated.capabilities,
            "node registered"
        );
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

        if name.trim().is_empty() {
            return Err(AppError::BadRequest("name must not be empty".into()));
        }

        if version.trim().is_empty() {
            return Err(AppError::BadRequest("version must not be empty".into()));
        }

        if self.services.values().any(|s|
            s.id != service_id &&
            s.name == name &&
            s.version == version
        ) {
            return Err(AppError::Conflict(
                format!("service '{}' version '{}' already exists", name, version)
            ));
        }

        if !self.services.contains_key(&service_id) {
            return Err(AppError::ServiceNotFound);
        }

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

    pub fn create_service(
        &mut self,
        name: String,
        version: String,
        selector: Option<String>,
    ) -> Result<ServiceDefinition, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest("name must not be empty".into()));
        }

        if version.trim().is_empty() {
            return Err(AppError::BadRequest("version must not be empty".into()));
        }

        if self
            .services
            .values()
            .any(|s| s.name == name && s.version == version)
        {
            return Err(AppError::Conflict(format!(
                "service '{}' version '{}' already exists",
                name, version
            )));
        }

        let service = ServiceDefinition {
            id: ServiceId::new(),
            name,
            version,
            selector,
        };

        self.services.insert(service.id, service.clone());

        tracing::info!(
            service_id = %service.id,
            name = %service.name,
            version = %service.version,
            "service created"
        );

        Ok(service)
    }

    pub async fn from_sqlite(stores: &SqliteStores) -> Result<Self, AppError> {
        let nodes = stores.node_repo.list().await?;
        let services = stores.service_repo.list().await?;
        let assignments = stores.assignment_repo.list().await?;

        let mut state = ControllerState::default();

        for node in nodes {
            state.nodes.insert(node.id, node);
        }

        for service in services {
            state.services.insert(service.id, service);
        }

        for assignment in assignments {
            state
                .assignments
                .entry(assignment.node_id)
                .or_default()
                .push(assignment.service_id);
        }

        let node_ids: Vec<NodeId> = state.nodes.keys().copied().collect();

        for node_id in node_ids {
            let reports = stores.observed_status_repo.list_for_node(node_id).await?;

            let observed: HashMap<ServiceId, ServiceStatus> = reports
                .into_iter()
                .map(|report| (report.service_id, report.status))
                .collect();

            if !observed.is_empty() {
                state.observed.insert(node_id, observed);
            }
        }

        for services in state.assignments.values_mut() {
            services.sort_by_key(|id| id.0);
            services.dedup();
        }

        tracing::info!(
            node_count = state.nodes.len(),
            service_count = state.services.len(),
            assignment_count = state.list_assignments().len(),
            "controller state restored from sqlite"
        );

        Ok(state)
    }
}

pub type SharedState = Arc<Mutex<ControllerState>>;

#[derive(Clone)]
pub struct AppState {
    pub inner: SharedState,
    pub storage: StorageMode,
}

impl AppState {
    pub fn new(storage: StorageMode) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ControllerState::default())),
            storage,
        }
    }

    pub fn in_memory() -> Self {
        Self::new(StorageMode::InMemory)
    }

    pub async fn register_node(
        &self,
        req: RegisterRequest,
    ) -> Result<NodeRecord, AppError> {
        let node = {
            let mut guard = self.inner.lock().await;
            guard.register_node(req)
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.node_repo.upsert(&node).await?;
        }

        Ok(node)
    }

    pub async fn record_heartbeat(
        &self,
        req: HeartbeatRequest,
    ) -> Result<NodeRecord, AppError> {
        let node_id = req.node_id;
        let timestamp = req.timestamp;
        let service_statuses = req.service_statuses.clone();

        let node = {
            let mut guard = self.inner.lock().await;
            guard.record_heartbeat(req)?
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores
                .node_repo
                .set_heartbeat(node_id, timestamp, node.status)
                .await?;

            stores
                .observed_status_repo
                .replace_for_node(node_id, &service_statuses)
                .await?;
        }

        Ok(node)
    }

    pub async fn create_service(
        &self,
        req: CreateServiceRequest,
    ) -> Result<ServiceDefinition, AppError> {
        let service = {
            let mut guard = self.inner.lock().await;

            let service = guard.create_service(
                req.name.trim().to_string(),
                req.version.trim().to_string(),
                req.selector.map(|s| s.trim().to_string()),
            )?;

            let nodes = guard.nodes.clone();
            let services = guard.services.clone();

            crate::domain::assignment_engine::assign_unassigned_services(
                &nodes,
                &services,
                &mut guard.assignments,
            );

            service
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.service_repo.insert(&service).await?;

            let assignments = {
                let guard = self.inner.lock().await;
                guard.list_assignments()
            };

            for assignment in assignments {
                if assignment.service_id == service.id {
                    stores
                        .assignment_repo
                        .assign(assignment.service_id, assignment.node_id)
                        .await?;
                }
            }
        }

        Ok(service)
    }

    pub async fn update_service(
        &self,
        service_id: ServiceId,
        req: UpdateServiceRequest,
    ) -> Result<ServiceDefinition, AppError> {
        let updated = {
            let mut guard = self.inner.lock().await;
            guard.update_service(
                service_id,
                req.name.trim().to_string(),
                req.version.trim().to_string(),
                req.selector.map(|s| s.trim().to_string()),
            )?
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.service_repo.update(&updated).await?;
        }

        Ok(updated)
    }

    pub async fn delete_service(
        &self,
        service_id: ServiceId,
    ) -> Result<(), AppError> {
        {
            let mut guard = self.inner.lock().await;
            guard.delete_service(service_id)?;
        }

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.service_repo.delete(service_id).await?;
        }

        Ok(())
    }

    pub async fn assign_service(
        &self,
        service_id: ServiceId,
        node_id: NodeId,
    ) -> Result<ServiceAssignment, AppError> {
        let assignment = {
            let mut guard = self.inner.lock().await;
            guard.assign_service(service_id, node_id)?
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.assignment_repo.assign(service_id, node_id).await?;
        }

        Ok(assignment)
    }

    pub async fn unassign_service(
        &self,
        service_id: ServiceId,
    ) -> Result<(), AppError> {
        {
            let mut guard = self.inner.lock().await;
            guard.unassign_service(service_id)?;
        }

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores.assignment_repo.unassign(service_id).await?;
        }

        Ok(())
    }

    pub async fn mark_node_offline(
        &self,
        node_id: NodeId,
    ) -> Result<(), AppError> {
        {
            let mut guard = self.inner.lock().await;
            let node = guard
                .nodes
                .get_mut(&node_id)
                .ok_or(AppError::NodeNotFound)?;

            node.status = NodeStatus::Offline;
        }

        if let StorageMode::Sqlite(stores) = &self.storage {
            stores
                .node_repo
                .set_status(node_id, NodeStatus::Offline)
                .await?;
        }

        Ok(())
    }

    pub async fn reassign_from_offline_nodes(
        &self,
    ) -> Result<Vec<ServiceId>, AppError> {
        let (unscheduled, assignments, offline_node_ids) = {
            let mut guard = self.inner.lock().await;

            let offline_node_ids: Vec<NodeId> = guard
                .nodes
                .values()
                .filter(|node| node.status == NodeStatus::Offline)
                .map(|node| node.id)
                .collect();

            let unscheduled =
                crate::background::reassignment_loop::reassign_from_offline_nodes(&mut guard);

            let assignments = guard.list_assignments();

            (unscheduled, assignments, offline_node_ids)
        };

        if let StorageMode::Sqlite(stores) = &self.storage {
            for offline_node_id in offline_node_ids {
                stores
                    .assignment_repo
                    .remove_for_node(offline_node_id)
                    .await?;

                stores
                    .observed_status_repo
                    .clear_for_node(offline_node_id)
                    .await?;
            }

            for assignment in assignments {
                stores
                    .assignment_repo
                    .assign(assignment.service_id, assignment.node_id)
                    .await?;
            }
        }

        Ok(unscheduled)
    }

    pub async fn from_sqlite(pool: SqlitePool) -> Result<Self, AppError> {
        let stores = SqliteStores::new(pool);
        let controller_state = ControllerState::from_sqlite(&stores).await?;

        Ok(Self {
            inner: Arc::new(Mutex::new(controller_state)),
            storage: StorageMode::Sqlite(stores),
        })
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
