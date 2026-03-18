use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid BIND_ADDRESS '{value}': {source}")]
    InvalidBindAddress {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },

    #[error("invalid HEARTBEAT_TIMEOUT_SECS '{value}': {source}")]
    InvalidHeartbeatTimeoutSecs {
        value: String,
        #[source]
        source: std::num::ParseIntError,
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
    pub bind_address: SocketAddr,
    pub heartbeat_timeout: Duration,
    pub poll_interval: Duration,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address_raw =
            env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        let bind_address = bind_address_raw
            .parse()
            .map_err(|source| ConfigError::InvalidBindAddress {
                value: bind_address_raw,
                source,
            })?;

        let heartbeat_timeout_secs = match env::var("HEARTBEAT_TIMEOUT_SECS") {
            Ok(raw) => raw
                .parse::<u64>()
                .map_err(|source| ConfigError::InvalidHeartbeatTimeoutSecs {
                    value: raw,
                    source,
                })?,
            Err(_) => 30,
        };

        let poll_interval_secs = match env::var("POLL_INTERVAL_SECS") {
            Ok(raw) => raw
                .parse::<u64>()
                .map_err(|source| ConfigError::InvalidPollIntervalSecs {
                    value: raw,
                    source,
                })?,
            Err(_) => 5,
        };

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            bind_address,
            heartbeat_timeout: Duration::from_secs(heartbeat_timeout_secs),
            poll_interval: Duration::from_secs(poll_interval_secs),
            log_level,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            heartbeat_timeout: Duration::from_secs(30),
            poll_interval: Duration::from_secs(5),
            log_level: "info".to_string(),
        }
    }
}
