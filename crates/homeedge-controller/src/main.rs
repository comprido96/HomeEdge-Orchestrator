mod app_state;
mod config;
mod domain;
mod error;
mod handlers;
mod observability;
mod repository;
mod router;

use homeedge_controller::{app_state::AppState, background::stale_node_watcher::run_stale_node_watcher, config::Config, observability::tracing::init_tracing, router::build_router};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    init_tracing(&config.log_level, &config.log_format);

    let state = AppState::new();

    let watcher_state = state.clone();
    let poll_interval = config.poll_interval;
    let heartbeat_timeout = config.heartbeat_timeout;

    tokio::spawn(async move {
        run_stale_node_watcher(watcher_state, poll_interval, heartbeat_timeout).await;
    });

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;
    tracing::info!(addr = %config.bind_address, "homeedge-controller listening");
    axum::serve(listener, app).await?;

    Ok(())
}
