use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::service::ServiceHealthReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for NodeId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NodeId(Uuid::parse_str(s)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Registering,
    Healthy,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: NodeId,
    pub status: NodeStatus,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub node_id: NodeId,
    pub timestamp: DateTime<Utc>,
    pub service_statuses: Vec<ServiceHealthReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub node_id: NodeId,
    pub capabilities: Vec<String>,
}
