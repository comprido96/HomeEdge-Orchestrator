use axum::{extract::State, http::StatusCode, Json};

use crate::app_state::AppState;
use homeedge_types::RegisterRequest;

pub async fn register(
    State(_state): State<AppState>,
    Json(_request): Json<RegisterRequest>,
) -> StatusCode {
    StatusCode::OK
}
