use axum::{extract::State, Json};

use crate::{app_state::AppState, error::AppError};
use homeedge_types::api::{HeartbeatRequest, HeartbeatResponse};

pub async fn heartbeat(
    State(state): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, AppError> {
    let services_for_log: Vec<String> = {
        let guard = state.inner.lock().await;

        req.service_statuses
            .iter()
            .map(|report| {
                let label = guard
                    .services
                    .get(&report.service_id)
                    .map(|svc| format!("{}-{}", svc.name, svc.version))
                    .unwrap_or_else(|| report.service_id.to_string());

                format!("{label}: {:?}", report.status)
            })
            .collect()
    };

    let node = state.record_heartbeat(req.clone()).await?;

    tracing::info!(
        node_id = %req.node_id,
        services = ?services_for_log,
        "heartbeat received"
    );

    Ok(Json(HeartbeatResponse { node }))
}
