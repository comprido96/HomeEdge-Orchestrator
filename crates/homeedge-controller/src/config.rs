use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    // Sprint 6: timeouts intentionally hardcoded; env config deferred post-demo
    pub bind_addr: SocketAddr,
    pub stale_node_timeout: Duration,
    pub reassignment_interval: Duration,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = env::var("BIND_ADDR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| "0.0.0.0:8080".parse().unwrap());

        Self {
            bind_addr,
            stale_node_timeout: Duration::from_secs(30),
            reassignment_interval: Duration::from_secs(5),
        }
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
