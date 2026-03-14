use std::time::Duration;

use tokio::time::{interval, MissedTickBehavior};

use crate::{
    app_state::SharedAgentAppState,
    controller_client::ControllerClient,
    error::AgentError,
};

pub async fn poll_once(
    client: &ControllerClient,
    state: &SharedAgentAppState,
) -> Result<(), AgentError> {
    let assignments = client.get_assignments().await?;

    let (changed, count) = {
        let mut guard = state.lock().await;
        let changed = guard.desired != assignments;
        let count = assignments.len();
        guard.desired = assignments;
        (changed, count)
    };

    if changed {
        tracing::info!(
            "assignments updated for node {}: count={}",
            client.node_id(),
            count
        );
    } else {
        tracing::info!(
            "assignments unchanged for node {}: count={}",
            client.node_id(),
            count
        );
    }

    Ok(())
}

pub async fn run_assignment_poll_loop(
    client: ControllerClient,
    state: SharedAgentAppState,
) {
    let mut ticker = interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;

        if let Err(err) = poll_once(&client, &state).await {
            tracing::error!(
                "assignment poll failed for node {}: {}",
                client.node_id(),
                err
            );
        }
    }
}