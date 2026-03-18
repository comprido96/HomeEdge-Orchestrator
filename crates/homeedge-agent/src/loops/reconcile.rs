use std::collections::{HashMap, HashSet};
use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use homeedge_types::service::{ServiceAssignment, ServiceDefinition, ServiceId};

use crate::{
    app_state::SharedAgentAppState,
    controller_client::ControllerClient,
    error::AgentError,
    runtime::service_runtime::ServiceManager,
};

const RECONCILE_INTERVAL: Duration = Duration::from_secs(5);

fn diff(
    desired: &HashSet<ServiceId>,
    local: &HashSet<ServiceId>,
) -> (Vec<ServiceId>, Vec<ServiceId>) {
    let mut to_start: Vec<ServiceId> = desired.difference(local).copied().collect();
    let mut to_stop: Vec<ServiceId> = local.difference(desired).copied().collect();
    to_start.sort_by_key(|id| id.0);
    to_stop.sort_by_key(|id| id.0);
    (to_start, to_stop)
}

pub async fn reconcile_once(
    client: &ControllerClient,
    state: &SharedAgentAppState,
    manager: &mut ServiceManager,
) -> Result<(), AgentError> {
    let assignments = client.get_assignments().await?;

    let assignments_changed = {
        let guard = state.lock().await;
        guard.desired != assignments
    };

    let services_by_id: HashMap<ServiceId, ServiceDefinition> = if assignments_changed {
        let services = client.list_services().await?;
        let by_id: HashMap<ServiceId, ServiceDefinition> = services
            .into_iter()
            .map(|svc| (svc.id, svc))
            .collect();

        {
            let mut guard = state.lock().await;
            guard.desired = assignments.clone();
            guard.services = by_id.clone();
        }

        by_id
    } else {
        let guard = state.lock().await;
        guard.services.clone()
    };

    let desired_ids: HashSet<ServiceId> = assignments
        .iter()
        .map(|a| a.service_id)
        .collect();

    let local_ids = manager.running_ids();

    let (to_start, to_stop) = diff(&desired_ids, &local_ids);

    if !to_start.is_empty() || !to_stop.is_empty() {
        tracing::info!(
            node_id = %client.node_id(),
            start_count = to_start.len(),
            stop_count = to_stop.len(),
            "reconciliation diff computed"
        );
    }

    for service_id in to_start {
        let Some(definition) = services_by_id.get(&service_id) else {
            tracing::warn!(node_id = %client.node_id(), service_id = %service_id,
                "assigned service missing definition; skipping start");
            continue;
        };

        if let Err(err) = manager.start(definition).await {
            tracing::error!(
                node_id = %client.node_id(),
                service_id = %service_id,
                error = %err,
                "failed to start service during reconciliation"
            );
        }
    }

    for service_id in to_stop {
        if let Err(err) = manager.stop(&service_id).await {
            tracing::error!(
                node_id = %client.node_id(),
                service_id = %service_id,
                error = %err,
                "failed to stop service during reconciliation"
            );
        }
    }

    let observed_statuses = manager.snapshot_statuses();

    {
        let mut guard = state.lock().await;
        guard.observed_statuses = observed_statuses.into_iter().collect();
    }

    Ok(())
}

pub async fn run_reconcile_loop(
    client: ControllerClient,
    state: SharedAgentAppState,
) {
    let mut ticker = interval(RECONCILE_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut manager = ServiceManager::new();

    loop {
        ticker.tick().await;

        if let Err(err) = reconcile_once(&client, &state, &mut manager).await {
            tracing::error!(
                node_id = %client.node_id(),
                error = %err,
                "reconciliation loop failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use homeedge_types::ServiceStatus;
    use tokio::sync::Mutex;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use homeedge_types::api::{AssignmentsResponse, ListServicesResponse};
    use homeedge_types::node::NodeId;
    use homeedge_types::service::{ServiceAssignment, ServiceDefinition, ServiceId};

    use crate::app_state::{AgentAppState, SharedAgentAppState};
    use crate::controller_client::ControllerClient;
    use crate::runtime::service_runtime::ServiceManager;

    use super::{diff, reconcile_once};

    // ------------------------------------------------------------------ //
    // Helpers
    // ------------------------------------------------------------------ //

    fn make_node_id() -> NodeId {
        NodeId(Uuid::new_v4())
    }

    fn make_service() -> ServiceDefinition {
        ServiceDefinition::new("lighting", "v1")
    }

    fn make_state(node_id: NodeId) -> SharedAgentAppState {
        Arc::new(Mutex::new(AgentAppState::new(node_id)))
    }

    fn make_client(base_url: &str, node_id: NodeId) -> ControllerClient {
        ControllerClient::new(base_url, node_id, vec![])
    }

    async fn mount_assignments(
        server: &MockServer,
        node_id: NodeId,
        assignments: Vec<ServiceAssignment>,
    ) {
        Mock::given(method("GET"))
            .and(path(format!("/assignments/{}", node_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(assignments.clone()))
            .mount(&server)
            .await;
    }

    async fn mount_services(server: &MockServer, services: Vec<ServiceDefinition>) {
        Mock::given(method("GET"))
            .and(path("/services"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ListServicesResponse {
                services,
            }))
            .mount(server)
            .await;
    }

    // ------------------------------------------------------------------ //
    // diff() unit tests
    // ------------------------------------------------------------------ //

    #[test]
    fn diff_empty_desired_and_local_is_noop() {
        let (to_start, to_stop) = diff(&HashSet::new(), &HashSet::new());
        assert!(to_start.is_empty());
        assert!(to_stop.is_empty());
    }

    #[test]
    fn diff_desired_has_new_service_produces_start() {
        let svc_id = ServiceId(Uuid::new_v4());
        let desired: HashSet<ServiceId> = [svc_id].into();
        let local: HashSet<ServiceId> = HashSet::new();

        let (to_start, to_stop) = diff(&desired, &local);
        assert_eq!(to_start, vec![svc_id]);
        assert!(to_stop.is_empty());
    }

    #[test]
    fn diff_local_has_extra_service_produces_stop() {
        let svc_id = ServiceId(Uuid::new_v4());
        let desired: HashSet<ServiceId> = HashSet::new();
        let local: HashSet<ServiceId> = [svc_id].into();

        let (to_start, to_stop) = diff(&desired, &local);
        assert!(to_start.is_empty());
        assert_eq!(to_stop, vec![svc_id]);
    }

    #[test]
    fn diff_same_desired_and_local_is_noop() {
        let svc_id = ServiceId(Uuid::new_v4());
        let desired: HashSet<ServiceId> = [svc_id].into();
        let local: HashSet<ServiceId> = [svc_id].into();

        let (to_start, to_stop) = diff(&desired, &local);
        assert!(to_start.is_empty());
        assert!(to_stop.is_empty());
    }

    // ------------------------------------------------------------------ //
    // reconcile_once() integration tests
    // ------------------------------------------------------------------ //

    #[tokio::test]
    async fn reconcile_once_starts_assigned_service() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        mount_assignments(
            &server,
            node_id,
            vec![ServiceAssignment { service_id: svc_id, node_id }],
        )
        .await;
        mount_services(&server, vec![svc]).await;

        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        assert_eq!(manager.len(), 1);

        manager.stop_all().await;
    }

    #[tokio::test]
    async fn reconcile_once_stops_service_no_longer_desired() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        // Start with the service running in the manager
        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();
        manager.start(&svc).await.unwrap();
        assert_eq!(manager.len(), 1);

        // Controller now returns empty assignments
        mount_assignments(&server, node_id, vec![]).await;
        mount_services(&server, vec![]).await;

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn reconcile_once_is_idempotent_when_assignments_unchanged() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        let assignments = vec![ServiceAssignment { service_id: svc_id, node_id }];

        Mock::given(method("GET"))
            .and(path(format!("/assignments/{}", node_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(assignments.clone()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ListServicesResponse {
                services: vec![svc],
            }))
            .expect(1)
            .mount(&server)
            .await;

        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();

        reconcile_once(&client, &state, &mut manager).await.unwrap();
        assert_eq!(manager.len(), 1);

        reconcile_once(&client, &state, &mut manager).await.unwrap();
        assert_eq!(manager.len(), 1);

        manager.stop_all().await;
    }

    #[tokio::test]
    async fn reconcile_once_skips_service_with_missing_definition() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc_id = ServiceId(Uuid::new_v4());

        // Assignment references a service_id that does not appear in list_services
        mount_assignments(
            &server,
            node_id,
            vec![ServiceAssignment { service_id: svc_id, node_id }],
        )
        .await;
        mount_services(&server, vec![]).await; // no definitions

        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        // Service was not started because definition was missing
        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn reconcile_once_publishes_observed_statuses() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        mount_assignments(
            &server,
            node_id,
            vec![ServiceAssignment { service_id: svc_id, node_id }],
        )
        .await;
        mount_services(&server, vec![svc]).await;

        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        let guard = state.lock().await;
        assert_eq!(
            guard.observed_statuses.get(&svc_id).copied(),
            Some(ServiceStatus::Running),
            "reconcile should publish Running status for started service"
        );
    }

    #[tokio::test]
    async fn reconcile_stops_service_when_assignment_is_removed() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();

        // Service is currently running locally
        manager.start(&svc).await.unwrap();
        assert_eq!(manager.len(), 1);

        // Controller no longer returns this service in assignments
        mount_assignments(&server, node_id, vec![]).await;
        mount_services(&server, vec![]).await;

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        // Service should have been stopped
        assert!(manager.is_empty());
        assert_eq!(
            manager.status(&svc_id),
            None,
            "service should no longer be tracked after assignment removal"
        );
    }

    #[tokio::test]
    async fn reconcile_starts_service_when_assignment_is_added() {
        let server = MockServer::start().await;
        let node_id = make_node_id();
        let svc = make_service();
        let svc_id = svc.id;

        // Service is not running locally — manager is empty
        let client = make_client(&server.uri(), node_id);
        let state = make_state(node_id);
        let mut manager = ServiceManager::new();
        assert!(manager.is_empty());

        // Controller now returns this service in assignments for this node
        // (simulates reassignment from a failed node)
        mount_assignments(
            &server,
            node_id,
            vec![ServiceAssignment { service_id: svc_id, node_id }],
        )
        .await;
        mount_services(&server, vec![svc]).await;

        reconcile_once(&client, &state, &mut manager).await.unwrap();

        // Service should now be running
        assert_eq!(manager.len(), 1);
        assert_eq!(
            manager.status(&svc_id),
            Some(ServiceStatus::Running),
            "reassigned service should be started by next reconcile tick"
        );

        manager.stop_all().await;
    }
}
