use std::env;

use homeedge_types::NodeId;
use thiserror::Error;
use uuid::Uuid;


pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 2;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid NODE_ID '{value}': {source}")]
    InvalidNodeId {
        value: String,
        #[source]
        source: uuid::Error,
    },

    #[error("invalid POLL_INTERVAL_SECS '{value}': {source}")]
    InvalidPollIntervalSecs {
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub controller_url: String,
    pub node_id: NodeId,
    pub poll_interval_secs: u64,
    pub log_level: String,
    pub log_format: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let controller_url =
            env::var("CONTROLLER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

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

        let poll_interval_secs = match env::var("POLL_INTERVAL_SECS") {
            Ok(raw) => raw
                .parse::<u64>()
                .map_err(|source| ConfigError::InvalidPollIntervalSecs {
                    value: raw,
                    source,
                })?,
            Err(_) => DEFAULT_POLL_INTERVAL_SECS,
        };

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let log_format = env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

        Ok(Self {
            controller_url,
            node_id,
            poll_interval_secs,
            log_level,
            log_format,
        })
    }
}
