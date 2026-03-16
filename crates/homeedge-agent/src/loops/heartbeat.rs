use std::time::Duration;

use chrono::Utc;
use tokio::time::{interval, MissedTickBehavior};

use crate::{
    app_state::SharedAgentAppState,
    controller_client::{ControllerClient, HeartbeatPayload},
};

pub async fn run_heartbeat_loop(
    client: ControllerClient,
    state: SharedAgentAppState,
) {
    let mut ticker = interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        let payload = {
            let guard = state.lock().await;

            let mut service_statuses: Vec<_> = guard
                .observed_statuses
                .iter()
                .map(|(service_id, status)| (*service_id, *status))
                .collect();

            service_statuses.sort_by_key(|(service_id, _)| service_id.0);

            HeartbeatPayload {
                service_statuses,
            }
        };

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
