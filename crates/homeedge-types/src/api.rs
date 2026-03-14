use serde::{Deserialize, Serialize};

use crate::NodeId;
use crate::node::{HeartbeatPayload, NodeRecord, RegistrationRequest};
use crate::service::ServiceId;

pub type RegisterRequest = RegistrationRequest;
pub type HeartbeatRequest = HeartbeatPayload;

/*
Storing only ServiceId internally is cleaner state, but it means the GET /assignments/{node_id} handler now needs to join against services
to build a useful response.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentsResponse {
    pub node_id: NodeId,
    pub service_ids: Vec<ServiceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesResponse {
    pub nodes: Vec<NodeRecord>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub node: NodeRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub node: NodeRecord,
}
