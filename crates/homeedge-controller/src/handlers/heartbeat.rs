use axum::{extract::State, http::StatusCode, Json};

use crate::app_state::AppState;
use homeedge_types::HeartbeatRequest;

pub async fn heartbeat(
    State(_state): State<AppState>,
    Json(_request): Json<HeartbeatRequest>,
) -> StatusCode {
    StatusCode::OK
}
