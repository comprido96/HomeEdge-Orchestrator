use std::time::Duration;

use tokio::time::{MissedTickBehavior, interval, sleep};

use crate::controller_client::{ControllerClient, HeartbeatPayload};

pub async fn run_heartbeat_loop(client: ControllerClient) {
    let mut ticker = interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let payload = HeartbeatPayload::default();

        match client.heartbeat(payload).await {
            Ok(()) => {
                tracing::info!("heartbeat sent for node {}", client.node_id());
            }
            Err(err) => {
                tracing::error!("heartbeat failed for node {}: {}", client.node_id(), err);
            }
        }
    }
}
