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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub id: ServiceId,
    pub name: String,
    pub version: String,
    pub selector: Option<String>,
}

impl ServiceDefinition {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: ServiceId::new(),
            name: name.into(),
            version: version.into(),
            selector: None,
        }
    }

    pub fn with_selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceAssignment {
    pub service_id: ServiceId,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_id_default_generates_uuid() {
        let a = ServiceId::default();
        let b = ServiceId::default();
        assert_ne!(a, b);
    }

    #[test]
    fn service_definition_new_sets_defaults() {
        let svc = ServiceDefinition::new("lighting", "v1");
        assert_eq!(svc.name, "lighting");
        assert_eq!(svc.version, "v1");
        assert_eq!(svc.selector, None);
    }

    #[test]
    fn service_definition_with_selector_sets_selector() {
        let svc = ServiceDefinition::new("lighting", "v1").with_selector("room=living");
        assert_eq!(svc.selector.as_deref(), Some("room=living"));
    }

    #[test]
    fn service_definition_serializes_and_deserializes() {
        let original = ServiceDefinition::new("lighting", "v1")
            .with_selector("room=living");
        let json = serde_json::to_string(&original).unwrap();
        let restored: ServiceDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
