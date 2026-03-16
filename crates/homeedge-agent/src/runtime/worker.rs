use std::time::Duration;

use homeedge_types::service::ServiceId;

pub async fn run_simulated_service(id: ServiceId, name: String) {
    tracing::info!(service_id = %id, service_name = %name, "worker loop started");

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tracing::debug!(service_id = %id, service_name = %name, "worker heartbeat");
    }
}
