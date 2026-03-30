use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use homeedge_types::{
    ServiceDefinition, ServiceId, ServiceStatus, node::NodeId, service::ServiceAssignment
};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAppState {
    pub node_id: NodeId,
    pub desired: Vec<ServiceAssignment>,
    pub services: HashMap<ServiceId, ServiceDefinition>,
    pub observed_statuses: HashMap<ServiceId, ServiceStatus>,
}

impl AgentAppState {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            desired: Vec::new(),
            services: HashMap::new(),
            observed_statuses: HashMap::new(),
        }
    }
}

pub type SharedAgentAppState = Arc<Mutex<AgentAppState>>;
