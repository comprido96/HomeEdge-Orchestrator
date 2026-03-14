use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::app_state::AppState;
use homeedge_types::NodeId;

pub async fn assignments(
    State(_state): State<AppState>,
    Path(_node_id): Path<NodeId>,
) -> StatusCode {
    StatusCode::OK
}
