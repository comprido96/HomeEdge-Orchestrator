use std::collections::{HashMap, HashSet};

use homeedge_types::node::{NodeId, NodeRecord, NodeStatus};
use homeedge_types::service::{ServiceDefinition, ServiceId};


pub fn assign_unassigned_services(
    nodes: &HashMap<NodeId, NodeRecord>,
    services: &HashMap<ServiceId, ServiceDefinition>,
    assignments: &mut HashMap<NodeId, Vec<ServiceId>>,
) {
    let healthy_nodes = sorted_healthy_node_ids(nodes);
    if healthy_nodes.is_empty() {
        return;
    }

    let assigned_services = currently_assigned_service_ids(assignments);

    let mut unassigned_service_ids: Vec<ServiceId> = services
        .keys()
        .copied()
        .filter(|service_id| !assigned_services.contains(service_id))
        .collect();

    unassigned_service_ids.sort_by_key(|id| id.0);

    for service_id in unassigned_service_ids {
        let node_id = healthy_nodes[0];
        assignments.entry(node_id).or_default().push(service_id);
    }

    for service_ids in assignments.values_mut() {
        service_ids.sort_by_key(|id| id.0);
        service_ids.dedup();
    }
}

fn sorted_healthy_node_ids(nodes: &HashMap<NodeId, NodeRecord>) -> Vec<NodeId> {
    let mut healthy: Vec<NodeId> = nodes
        .iter()
        .filter_map(|(node_id, record)| {
            (record.status == NodeStatus::Healthy).then_some(*node_id)
        })
        .collect();

    healthy.sort_by_key(|id| id.0);
    healthy
}

fn currently_assigned_service_ids(
    assignments: &HashMap<NodeId, Vec<ServiceId>>,
) -> HashSet<ServiceId> {
    assignments
        .values()
        .flat_map(|service_ids| service_ids.iter().copied())
        .collect()
}


pub struct AssignmentEngine;

impl AssignmentEngine {
    pub fn reconcile(
        nodes: &HashMap<NodeId, NodeRecord>,
        services: &HashMap<ServiceId, ServiceDefinition>,
        assignments: &mut HashMap<NodeId, Vec<ServiceId>>,
    ) {
        assign_unassigned_services(nodes, services, assignments);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use homeedge_types::node::{NodeRecord, NodeStatus};
    use homeedge_types::service::ServiceDefinition;

    fn healthy_node() -> NodeRecord {
        NodeRecord {
            id: NodeId::new(),
            status: NodeStatus::Healthy,
            last_heartbeat: Some(Utc::now()),
            capabilities: vec![],
        }
    }

    fn registering_node() -> NodeRecord {
        NodeRecord {
            id: NodeId::new(),
            status: NodeStatus::Registering,
            last_heartbeat: None,
            capabilities: vec![],
        }
    }

    fn service(name: &str) -> (ServiceId, ServiceDefinition) {
        let svc = ServiceDefinition::new(name, "v1");
        (svc.id, svc)
    }

    #[test]
    fn assigns_unassigned_services_to_first_healthy_node() {
        let node_a = healthy_node();
        let node_b = healthy_node();

        let node_a_id = node_a.id;
        let node_b_id = node_b.id;

        let mut nodes = HashMap::new();
        nodes.insert(node_a_id, node_a);
        nodes.insert(node_b_id, node_b);

        let (svc1_id, svc1) = service("lighting");
        let (svc2_id, svc2) = service("hvac");

        let mut services = HashMap::new();
        services.insert(svc1_id, svc1);
        services.insert(svc2_id, svc2);

        let mut assignments: HashMap<NodeId, Vec<ServiceId>> = HashMap::new();

        assign_unassigned_services(&nodes, &services, &mut assignments);

        // first-fit: all unassigned services go to the lowest-sorted healthy node
        let expected_node = [node_a_id, node_b_id]
            .iter()
            .min_by_key(|id| id.0)
            .copied()
            .unwrap();

        let assigned_to_first = assignments.get(&expected_node).unwrap();
        assert_eq!(assigned_to_first.len(), 2);

        assert!(assigned_to_first.contains(&svc1_id));
        assert!(assigned_to_first.contains(&svc2_id));

        let total_assigned: usize = assignments.values().map(Vec::len).sum();
        assert_eq!(total_assigned, 2);
    }

    #[test]
    fn does_not_assign_when_no_healthy_nodes_exist() {
        let node = registering_node();
        let node_id = node.id;

        let mut nodes = HashMap::new();
        nodes.insert(node_id, node);

        let (svc_id, svc) = service("lighting");
        let mut services = HashMap::new();
        services.insert(svc_id, svc);

        let mut assignments = HashMap::new();

        assign_unassigned_services(&nodes, &services, &mut assignments);

        assert!(assignments.is_empty());
    }

    #[test]
    fn does_not_reassign_already_assigned_services() {
        let node_a = healthy_node();
        let node_b = healthy_node();

        let node_a_id = node_a.id;
        let node_b_id = node_b.id;

        let mut nodes = HashMap::new();
        nodes.insert(node_a_id, node_a);
        nodes.insert(node_b_id, node_b);

        let (svc_id, svc) = service("lighting");
        let mut services = HashMap::new();
        services.insert(svc_id, svc);

        let mut assignments = HashMap::new();
        assignments.insert(node_b_id, vec![svc_id]);

        assign_unassigned_services(&nodes, &services, &mut assignments);

        let total_assigned: usize = assignments.values().map(Vec::len).sum();
        assert_eq!(total_assigned, 1);
        assert_eq!(assignments.get(&node_b_id), Some(&vec![svc_id]));
    }

    #[test]
    fn only_unassigned_services_are_placed() {
        let node = healthy_node();
        let node_id = node.id;

        let mut nodes = HashMap::new();
        nodes.insert(node_id, node);

        let (svc1_id, svc1) = service("lighting");
        let (svc2_id, svc2) = service("hvac");

        let mut services = HashMap::new();
        services.insert(svc1_id, svc1);
        services.insert(svc2_id, svc2);

        let mut assignments = HashMap::new();
        assignments.insert(node_id, vec![svc1_id]);

        assign_unassigned_services(&nodes, &services, &mut assignments);

        let assigned = assignments.get(&node_id).unwrap();
        assert_eq!(assigned.len(), 2);
        assert!(assigned.contains(&svc1_id));
        assert!(assigned.contains(&svc2_id));
    }

    #[test]
    fn calling_engine_twice_is_idempotent() {
        let node = healthy_node();
        let node_id = node.id;
        let mut nodes = HashMap::new();
        nodes.insert(node_id, node);

        let (svc_id, svc) = service("lighting");
        let mut services = HashMap::new();
        services.insert(svc_id, svc);

        let mut assignments = HashMap::new();
        assign_unassigned_services(&nodes, &services, &mut assignments);
        assign_unassigned_services(&nodes, &services, &mut assignments);

        let total: usize = assignments.values().map(Vec::len).sum();
        assert_eq!(total, 1);
    }
}
