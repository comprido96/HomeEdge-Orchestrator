mod app_state;
mod config;
mod controller_client;
mod error;
mod loops;
mod observability;
mod runtime;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    app_state::{AgentAppState, SharedAgentAppState},
    config::Config,
    controller_client::ControllerClient,
    loops::{
        heartbeat::run_heartbeat_loop,
        reconcile::run_reconcile_loop,
        registration::wait_until_registered,
    },
    observability::tracing::init_tracing,
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    init_tracing(&config.log_level, &config.log_format);

    tracing::info!(
        node_id = %config.node_id,
        controller = %config.controller_url,
        poll_interval_secs = config.poll_interval_secs,
        "agent starting"
    );

    let state: SharedAgentAppState =
        Arc::new(Mutex::new(AgentAppState::new(config.node_id)));

    let client = ControllerClient::new(
        &config.controller_url,
        config.node_id,
        vec!["docker".into(), "mqtt".into()],
    );

    wait_until_registered(&client).await;

    let heartbeat_client = client.clone();
    let heartbeat_state = state.clone();
    let heartbeat_interval_secs = config.poll_interval_secs;
    let heartbeat_task = tokio::spawn(async move {
        run_heartbeat_loop(heartbeat_client, heartbeat_state, heartbeat_interval_secs).await;
    });

    let reconcile_client = client.clone();
    let reconcile_state = state.clone();
    let reconcile_interval_secs = config.poll_interval_secs;
    let reconcile_task = tokio::spawn(async move {
        run_reconcile_loop(reconcile_client, reconcile_state, reconcile_interval_secs).await;
    });

    let _ = tokio::join!(heartbeat_task, reconcile_task);

    Ok(())
}
