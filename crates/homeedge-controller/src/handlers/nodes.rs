use axum::{extract::State, http::StatusCode};

use crate::app_state::AppState;

pub async fn nodes(State(_state): State<AppState>) -> StatusCode {
    StatusCode::OK
}
