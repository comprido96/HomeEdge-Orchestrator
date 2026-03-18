use std::env;

use homeedge_types::NodeId;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid NODE_ID '{value}': {source}")]
    InvalidNodeId {
        value: String,
        #[source]
        source: uuid::Error,
    },

    #[error("invalid HEARTBEAT_INTERVAL '{value}': {source}")]
    InvalidHeartbeatInterval {
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub controller_url: String,
    pub node_id: NodeId,
    pub heartbeat_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let controller_url =
            env::var("CONTROLLER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

        let node_id = match env::var("NODE_ID") {
            Ok(raw) => {
                let parsed = raw
                    .parse::<Uuid>()
                    .map_err(|source| ConfigError::InvalidNodeId {
                        value: raw,
                        source,
                    })?;
                NodeId(parsed)
            }
            Err(_) => {
                tracing::info!("NODE_ID not set, generating fresh NodeId");
                NodeId::new()
            }
        };

        let heartbeat_interval_secs = match env::var("HEARTBEAT_INTERVAL") {
            Ok(raw) => raw
                .parse::<u64>()
                .map_err(|source| ConfigError::InvalidHeartbeatInterval {
                    value: raw,
                    source,
                })?,
            Err(_) => 2,
        };

        Ok(Self {
            controller_url,
            node_id,
            heartbeat_interval_secs,
        })
    }
}
