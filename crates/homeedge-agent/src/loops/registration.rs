use std::time::Duration;

use tokio::time::sleep;

use crate::controller_client::ControllerClient;
use crate::error::AgentError;

pub async fn wait_until_registered(client: &ControllerClient) -> () {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        match client.register().await {
            Ok(()) => {
                tracing::info!("registered node {} with controller", client.node_id());
                return ();
            }
            Err(err) => {
                tracing::error!(
                    "registration failed for node {}: {}. retrying in {:?}",
                    client.node_id(),
                    err,
                    backoff
                );

                sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, max_backoff);
            }
        }
    }
}
