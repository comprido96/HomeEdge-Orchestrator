use axum::{extract::State, Json};
use crate::{app_state::AppState, error::AppError};
use homeedge_types::api::NodesResponse;


pub async fn list_nodes(
    State(state): State<AppState>,
) -> Result<Json<NodesResponse>, AppError> {
    let guard = state.inner.lock().await;
    let nodes = guard.list_node_views();

    Ok(Json(NodesResponse { nodes }))
}
