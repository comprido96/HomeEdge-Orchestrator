use axum::{
    extract::{Path, State},
    Json,
};

use crate::app_state::AppState;
use crate::error::AppError;
use homeedge_types::api::AssignmentsResponse;
use homeedge_types::node::NodeId;

pub async fn get_assignments(
    State(state): State<AppState>,
    Path(node_id): Path<NodeId>,
) -> Result<Json<AssignmentsResponse>, AppError> {
    let guard = state.inner.lock().await;
    let service_ids = guard.assignments_for(node_id)?;

    Ok(Json(AssignmentsResponse {
        node_id,
        service_ids,
    }))
}
