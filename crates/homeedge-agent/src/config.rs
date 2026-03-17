use std::env;
use homeedge_types::NodeId;
use uuid::Uuid;


#[derive(Debug, Clone)]
pub struct Config {
    pub controller_url: String,
    pub node_id: NodeId,
    pub heartbeat_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let controller_url =
            env::var("CONTROLLER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

        let node_id = env::var("NODE_ID")
            .ok()
            .and_then(|s| s.parse::<Uuid>().ok())
            .map(NodeId)
            .unwrap_or_else(|| {
                tracing::info!("NODE_ID not set or invalid, generating fresh NodeId");
                NodeId::new()
            });

        let heartbeat_interval_secs =
            env::var("HEARTBEAT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2);

        Self {
            controller_url,
            node_id,
            heartbeat_interval_secs,
        }
    }
}
