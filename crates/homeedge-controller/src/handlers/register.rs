use axum::{extract::State, Json};

use crate::app_state::AppState;
use crate::error::AppError;
use homeedge_types::api::{RegisterRequest, RegisterResponse};
use homeedge_types::node::{NodeRecord, NodeStatus};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let mut guard = state.inner.lock().await;
    let node = guard.register_node(req);

    Ok(Json(RegisterResponse { node }))
}
