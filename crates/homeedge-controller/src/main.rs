use std::net::SocketAddr;

use homeedge_controller::{
    app_state::{AppState, SqliteStores, StorageMode},
    background::stale_node_watcher::run_stale_node_watcher,
    config::{Config, StorageBackend},
    router::build_router,
};

use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;

    let state = match config.storage_backend {
        StorageBackend::InMemory => {
            tracing::info!("starting controller with in-memory storage");
            AppState::in_memory()
        }
        StorageBackend::Sqlite => {
            tracing::info!(
                database_url = %config.sqlite_database_url,
                "starting controller with sqlite storage"
            );

            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&config.sqlite_database_url)
                .await?;

            sqlx::migrate!("./migrations").run(&pool).await?;

            AppState::from_sqlite(pool).await?
        }
    };

    let watcher_state = state.clone();
    let poll_interval = config.poll_interval;
    let heartbeat_timeout = config.heartbeat_timeout;

    tokio::spawn(async move {
        run_stale_node_watcher(watcher_state, poll_interval, heartbeat_timeout).await;
    });

    let app = build_router(state);

    let addr: SocketAddr = config.bind_address;

    tracing::info!(%addr, "homeedge-controller listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
