use homeedge_types::{NodeStatus, ServiceId};

use crate::app_state::ControllerState;

/// Reassign all services currently owned by offline nodes to the first healthy node available.
/// Returns the list of service IDs that could not be placed anywhere.
pub fn reassign_from_offline_nodes(state: &mut ControllerState) -> Vec<ServiceId> {
    let offline_node_ids: Vec<_> = state
        .nodes
        .values()
        .filter(|node| node.status == NodeStatus::Offline)
        .map(|node| node.id)
        .collect();

    let mut unscheduled = Vec::new();

    for offline_node_id in offline_node_ids {
        let orphaned = state.assignments.remove(&offline_node_id).unwrap_or_default();
        
        // Clear stale heartbeat-reported statuses from the offline node
        state.observed.remove(&offline_node_id);

        for service_id in orphaned {
            let mut healthy_nodes: Vec<_> = state.nodes.values()
                .filter(|n| n.status == NodeStatus::Healthy && n.id != offline_node_id)
                .collect();
            healthy_nodes.sort_by_key(|n| n.id.0);
            let target_node_id = healthy_nodes.first().map(|n| n.id);

            match target_node_id {
                Some(target_node_id) => {
                    let target_services = state.assignments.entry(target_node_id).or_default();

                    if !target_services.contains(&service_id) {
                        target_services.push(service_id);
                    }

                    tracing::info!(
                        service_id = %service_id,
                        from_node = %offline_node_id,
                        to_node = %target_node_id,
                        "service reassigned"
                    );
                }
                None => {
                    tracing::error!(
                        service_id = %service_id,
                        from_node = %offline_node_id,
                        "no healthy node available for reassignment"
                    );
                    unscheduled.push(service_id);
                }
            }
        }
    }

    unscheduled
}

#[cfg(test)]
mod tests {
    use homeedge_types::{
        node::{NodeId, NodeRecord, NodeStatus},
        service::{ServiceDefinition, ServiceId},
    };
    use uuid::Uuid;

    use crate::app_state::ControllerState;

    use super::reassign_from_offline_nodes;

    fn node_id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    fn service_id(n: u128) -> ServiceId {
        ServiceId(Uuid::from_u128(n))
    }

    fn node(id: NodeId, status: NodeStatus) -> NodeRecord {
        NodeRecord {
            id,
            status,
            last_heartbeat: None,
            capabilities: vec!["docker".into()],
        }
    }

    #[test]
    fn reassigns_services_from_offline_node_to_healthy_node() {
        let offline = node_id(1);
        let healthy = node_id(2);
        let svc = service_id(10);

        let mut state = ControllerState::default();
        state.nodes.insert(offline, node(offline, NodeStatus::Offline));
        state.nodes.insert(healthy, node(healthy, NodeStatus::Healthy));
        state.services.insert(svc, ServiceDefinition::new("lighting", "v1"));
        state.assignments.insert(offline, vec![svc]);

        let unscheduled = reassign_from_offline_nodes(&mut state);

        assert!(unscheduled.is_empty());
        assert_eq!(state.assignments.get(&offline), None);
        assert_eq!(state.assignments.get(&healthy), Some(&vec![svc]));
    }

    #[test]
    fn keeps_service_unscheduled_when_no_healthy_node_exists() {
        let offline = node_id(1);
        let svc = service_id(10);

        let mut state = ControllerState::default();
        state.nodes.insert(offline, node(offline, NodeStatus::Offline));
        state.services.insert(svc, ServiceDefinition::new("lighting", "v1"));
        state.assignments.insert(offline, vec![svc]);

        let unscheduled = reassign_from_offline_nodes(&mut state);

        assert_eq!(unscheduled, vec![svc]);
        assert_eq!(state.assignments.get(&offline), None);
    }

    #[test]
    fn ignores_offline_nodes_with_no_assignments() {
        let offline = node_id(1);
        let healthy = node_id(2);

        let mut state = ControllerState::default();
        state.nodes.insert(offline, node(offline, NodeStatus::Offline));
        state.nodes.insert(healthy, node(healthy, NodeStatus::Healthy));

        let unscheduled = reassign_from_offline_nodes(&mut state);

        assert!(unscheduled.is_empty());
        assert!(state.assignments.get(&healthy).is_none());
    }

    #[test]
    fn reassign_is_idempotent_when_called_twice() {
        let offline = node_id(1);
        let healthy = node_id(2);
        let svc = service_id(10);

        let mut state = ControllerState::default();
        state.nodes.insert(offline, node(offline, NodeStatus::Offline));
        state.nodes.insert(healthy, node(healthy, NodeStatus::Healthy));
        state.services.insert(svc, ServiceDefinition::new("lighting", "v1"));
        state.assignments.insert(offline, vec![svc]);

        reassign_from_offline_nodes(&mut state);
        reassign_from_offline_nodes(&mut state);

        let assignments = state.assignments.get(&healthy).unwrap();
        assert_eq!(assignments.len(), 1);
    }
}
