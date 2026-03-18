mod app_state;
mod config;
mod domain;
mod error;
mod handlers;
mod observability;
mod repository;
mod router;

use homeedge_controller::{app_state::AppState, background::stale_node_watcher::run_stale_node_watcher, config::Config, router::build_router};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let state = AppState::new();

    let watcher_state = state.clone();
    let watcher_interval = config.reassignment_interval;
    let stale_timeout = config.stale_node_timeout;

    tokio::spawn(async move {
        run_stale_node_watcher(watcher_state, watcher_interval, stale_timeout).await;
    });


    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "homeedge-controller listening");
    axum::serve(listener, app).await?;

    Ok(())
}
