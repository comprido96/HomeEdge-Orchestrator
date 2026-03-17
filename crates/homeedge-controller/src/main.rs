mod app_state;
mod config;
mod domain;
mod error;
mod handlers;
mod observability;
mod repository;
mod router;

use homeedge_controller::{app_state::AppState, config::Config, router::build_router};
use std::net::SocketAddr;
use homeedge_controller::background::stale_node_watcher::run_stale_node_watcher;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();
    let state = AppState::new();

    let watcher_state = state.clone();
    tokio::spawn(async move {
        run_stale_node_watcher(
            watcher_state,
            config.stale_node_timeout,
            config.reassignment_interval,
        )
        .await;
    });

    let app = build_router(state);

    let addr: SocketAddr = config.bind_addr;

    tracing::info!(%addr, "homeedge-controller listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("controller server failed");
}
