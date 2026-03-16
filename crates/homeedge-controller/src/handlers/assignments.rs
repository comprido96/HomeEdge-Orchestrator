use axum::{
    Json, extract::{Path, State}, http::StatusCode
};

use crate::app_state::AppState;
use crate::error::AppError;
use homeedge_types::{ServiceAssignment, ServiceId, api::AssignServiceRequest, node::NodeId};


pub async fn get_assignments(
    State(state): State<AppState>,
    Path(node_id): Path<NodeId>,
) -> Result<Json<Vec<ServiceAssignment>>, AppError> {
    let guard = state.inner.lock().await;
    let assignments = guard.assignments_for(node_id)?;
    Ok(Json(assignments))
}


pub async fn list_assignments(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServiceAssignment>>, AppError> {

    let guard = state.inner.lock().await;

    Ok(Json(guard.list_assignments()))
}


pub async fn assign_service(
    State(state): State<AppState>,
    Path(service_id): Path<ServiceId>,
    Json(req): Json<AssignServiceRequest>,
) -> Result<Json<ServiceAssignment>, AppError> {

    let mut guard = state.inner.lock().await;

    let assignment = guard.assign_service(service_id, req.node_id)?;

    Ok(Json(assignment))
}


pub async fn unassign_service(
    State(state): State<AppState>,
    Path(service_id): Path<ServiceId>,
) -> Result<StatusCode, AppError> {

    let mut guard = state.inner.lock().await;

    guard.unassign_service(service_id)?;

    Ok(StatusCode::NO_CONTENT)
}


pub async fn get_assignments_for_node(
    State(state): State<AppState>,
    Path(node_id): Path<NodeId>,
) -> Result<Json<Vec<ServiceAssignment>>, AppError> {

    let guard = state.inner.lock().await;

    Ok(Json(guard.assignments_for(node_id)?))
}


#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use homeedge_types::{AssignmentsResponse, NodeId, NodeRecord, NodeStatus, ServiceAssignment, ServiceDefinition};

    use crate::{app_state::{AppState, ControllerState}, router::build_router};

    fn test_state() -> AppState {
        AppState {
            inner: Arc::new(Mutex::new(ControllerState::default())),
        }
    }

    #[tokio::test]
    async fn get_assignments_returns_empty_vec_for_existing_node_with_no_assignments() {
        let node_id = NodeId::new();

        let mut controller_state = ControllerState::default();
        controller_state.nodes.insert(
            node_id,
            NodeRecord {
                id: node_id,
                status: NodeStatus::Healthy,
                last_heartbeat: None,
                capabilities: vec![],
            },
        );

        let app = build_router(AppState {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(controller_state)),
        });

        let req = Request::builder()
            .method("GET")
            .uri(format!("/assignments/{node_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_assignments_returns_real_assignments_for_node() {
        let node_id = NodeId::new();

        let svc = ServiceDefinition::new("lighting", "v1");
        let service_id = svc.id;

        let mut controller_state = ControllerState::default();
        controller_state.nodes.insert(
            node_id,
            NodeRecord {
                id: node_id,
                status: NodeStatus::Healthy,
                last_heartbeat: None,
                capabilities: vec![],
            },
        );
        controller_state.services.insert(service_id, svc);
        controller_state.assignments.insert(node_id, vec![service_id]);

        let app = build_router(AppState {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(controller_state)),
        });

        let req = Request::builder()
            .method("GET")
            .uri(format!("/assignments/{node_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: Vec<ServiceAssignment> = serde_json::from_slice(&body).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].service_id, service_id);
        assert_eq!(result[0].node_id, node_id);
    }

    #[tokio::test]
    async fn get_assignments_returns_not_found_for_unknown_node() {
        let app = build_router(test_state());

        let req = Request::builder()
            .method("GET")
            .uri(format!("/assignments/{}", NodeId::new()))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
