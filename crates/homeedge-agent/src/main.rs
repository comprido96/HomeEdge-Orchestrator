mod controller_client;
mod loops;
mod observability;
mod runtime;

use tokio::time::{sleep, Duration};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("homeedge-agent starting");

    sleep(Duration::from_secs(1)).await;

    tracing::info!("homeedge-agent exiting");
}
