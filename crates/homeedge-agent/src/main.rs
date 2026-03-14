mod controller_client;
mod error;
mod loops;
mod observability;
mod runtime;

use homeedge_types::NodeId;
use crate::controller_client::ControllerClient;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::loops::{assignment_poll::run_assignment_poll_loop, heartbeat::run_heartbeat_loop, registration::wait_until_registered};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("homeedge-agent starting");

    let node_id = NodeId::new();

    let client = ControllerClient::new(
        "http://127.0.0.1:8080",
        node_id,
        vec!["docker".into(), "mqtt".into()],
    );

    wait_until_registered(&client).await;

    let heartbeat_client = client.clone();
    let assignment_client = client.clone();

    let heartbeat_task = tokio::spawn(async move {
        run_heartbeat_loop(heartbeat_client).await;
    });

    let assignment_task = tokio::spawn(async move {
        run_assignment_poll_loop(assignment_client).await;
    });

    // TODO: handle task panics explicitly in a later sprint
    let _ = tokio::join!(heartbeat_task, assignment_task);

    Ok(())
}
