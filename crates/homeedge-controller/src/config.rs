use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub stale_node_timeout: Duration,
    pub reassignment_interval: Duration,
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
