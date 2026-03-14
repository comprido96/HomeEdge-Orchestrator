use serde::{Deserialize, Serialize};

use crate::node::{HeartbeatPayload, NodeRecord, RegistrationRequest};
use crate::service::ServiceAssignment;

pub type RegisterRequest = RegistrationRequest;
pub type HeartbeatRequest = HeartbeatPayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentsResponse {
    pub assignments: Vec<ServiceAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesResponse {
    pub nodes: Vec<NodeRecord>,
}
