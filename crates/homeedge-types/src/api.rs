use serde::{Deserialize, Serialize};

use crate::{NodeId, ServiceDefinition};
use crate::node::{HeartbeatPayload, NodeRecord, RegistrationRequest};
use crate::service::{ServiceHealthReport, ServiceId};

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
pub struct NodeView {
    pub node: NodeRecord,
    pub services: Vec<ServiceHealthReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesResponse {
    pub nodes: Vec<NodeView>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub node: NodeRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub node: NodeRecord,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub version: String,
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServiceResponse {
    pub service: ServiceDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListServicesResponse {
    pub services: Vec<ServiceDefinition>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignServiceRequest {
    pub node_id: NodeId,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: String,
    pub version: String,
    pub selector: Option<String>,
}
