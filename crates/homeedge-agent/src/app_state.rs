use std::sync::Arc;

use tokio::sync::Mutex;

use homeedge_types::{
    node::NodeId,
    service::ServiceAssignment,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAppState {
    pub node_id: NodeId,
    pub desired: Vec<ServiceAssignment>,
}

impl AgentAppState {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            desired: Vec::new(),
        }
    }
}

pub type SharedAgentAppState = Arc<Mutex<AgentAppState>>;
