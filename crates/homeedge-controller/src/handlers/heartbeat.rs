use axum::{extract::State, Json};

use crate::app_state::AppState;
use crate::error::AppError;
use homeedge_types::api::{HeartbeatRequest, HeartbeatResponse};
use homeedge_types::node::NodeStatus;

pub async fn heartbeat(
    State(state): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, AppError> {
    let mut guard = state.inner.lock().await;
    let node = guard.record_heartbeat(req)?;

    Ok(Json(HeartbeatResponse { node }))
}
