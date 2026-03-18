use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid BIND_ADDR '{value}': {source}")]
    InvalidBindAddr {
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },
}


#[derive(Debug, Clone)]
pub struct Config {
    // Sprint 6: timeouts intentionally hardcoded; env config deferred post-demo
    pub bind_addr: SocketAddr,
    pub stale_node_timeout: Duration,
    pub reassignment_interval: Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr_raw =
            env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        let bind_addr = bind_addr_raw
            .parse()
            .map_err(|source| ConfigError::InvalidBindAddr {
                value: bind_addr_raw,
                source,
            })?;

        Ok(Self {
            bind_addr,
            stale_node_timeout: Duration::from_secs(30),
            reassignment_interval: Duration::from_secs(5),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            stale_node_timeout: Duration::from_secs(30),
            reassignment_interval: Duration::from_secs(5),
        }
    }
}
