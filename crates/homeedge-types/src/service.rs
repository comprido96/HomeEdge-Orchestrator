use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::node::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServiceId(pub Uuid);

impl ServiceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ServiceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ServiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub id: ServiceId,
    pub name: String,
    pub version: String,
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAssignment {
    pub service_id: ServiceId,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Assigned,
    Starting,
    Running,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthReport {
    pub service_id: ServiceId,
    pub status: ServiceStatus,
}
