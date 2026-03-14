use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, sleep};

use crate::controller_client::ControllerClient;

pub async fn run_assignment_poll_loop(client: ControllerClient) {
    let mut ticker = interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        match client.get_assignments().await {
            Ok(assignments) => {
                tracing::info!(
                    "assignments for node {}: {:?}",
                    client.node_id(),
                    assignments
                );
            }
            Err(err) => {
                tracing::error!(
                    "assignment poll failed for node {}: {}",
                    client.node_id(),
                    err
                );
            }
        }
    }
}
