mod app_state;
mod controller_client;
mod error;
mod loops;
mod observability;
mod runtime;

use std::sync::Arc;

use tokio::sync::Mutex;

use homeedge_types::NodeId;

use crate::{
    app_state::{AgentAppState, SharedAgentAppState},
    controller_client::ControllerClient,
    loops::{
        heartbeat::run_heartbeat_loop, reconcile::run_reconcile_loop, registration::wait_until_registered
    },
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("homeedge-agent starting");

    let node_id = NodeId::new();

    let state: SharedAgentAppState =
        Arc::new(Mutex::new(AgentAppState::new(node_id)));

    let client = ControllerClient::new(
        "http://127.0.0.1:8080",
        node_id,
        vec!["docker".into(), "mqtt".into()],
    );

    wait_until_registered(&client).await;

    let heartbeat_client = client.clone();
    let heartbeat_state = state.clone();
    let heartbeat_task = tokio::spawn(async move {
        run_heartbeat_loop(heartbeat_client, heartbeat_state).await;
    });

    let reconcile_client = client.clone();
    let reconcile_state = state.clone();
    let reconcile_task = tokio::spawn(async move {
        run_reconcile_loop(reconcile_client, reconcile_state).await;
    });

    let _ = tokio::join!(heartbeat_task, reconcile_task);

    Ok(())
}