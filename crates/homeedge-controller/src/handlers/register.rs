use axum::{extract::State, Json};

use crate::{app_state::AppState, error::AppError};
use homeedge_types::api::{RegisterRequest, RegisterResponse};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let node = state.register_node(req).await?;

    Ok(Json(RegisterResponse { node }))
}
