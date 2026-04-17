use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use homeedge_types::{
    api::{CreateServiceRequest, CreateServiceResponse, ListServicesResponse, UpdateServiceRequest},
    service::ServiceDefinition,
    ServiceId,
};

use crate::{app_state::AppState, error::AppError};

pub async fn create_service(
    State(state): State<AppState>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<(StatusCode, Json<CreateServiceResponse>), AppError> {
    let service = state.create_service(req).await?;

    Ok((StatusCode::CREATED, Json(CreateServiceResponse { service })))
}

pub async fn list_services(
    State(state): State<AppState>,
) -> Result<Json<ListServicesResponse>, AppError> {
    let guard = state.inner.lock().await;

    let mut services: Vec<ServiceDefinition> =
        guard.services.values().cloned().collect();

    services.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.version.cmp(&b.version))
            .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
    });

    Ok(Json(ListServicesResponse { services }))
}

pub async fn get_service(
    State(state): State<AppState>,
    Path(service_id): Path<ServiceId>,
) -> Result<Json<ServiceDefinition>, AppError> {
    let guard = state.inner.lock().await;
    let service = guard.get_service(service_id)?;
    Ok(Json(service))
}

pub async fn delete_service(
    State(state): State<AppState>,
    Path(service_id): Path<ServiceId>,
) -> Result<StatusCode, AppError> {
    state.delete_service(service_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_service(
    State(state): State<AppState>,
    Path(service_id): Path<ServiceId>,
    Json(req): Json<UpdateServiceRequest>,
) -> Result<Json<ServiceDefinition>, AppError> {
    let updated = state.update_service(service_id, req).await?;
    Ok(Json(updated))
}


#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use homeedge_types::{
        node::{NodeId, NodeRecord, NodeStatus},
        ListServicesResponse,
    };
    use tower::ServiceExt;

    use crate::{
        app_state::{AppState, ControllerState, StorageMode},
        router::build_router,
    };

    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn test_state() -> AppState {
        AppState {
            inner: Arc::new(Mutex::new(ControllerState::default())),
            storage: StorageMode::InMemory,
        }
    }

    #[tokio::test]
    async fn post_services_creates_service() {
        let app = build_router(test_state());

        let req = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"lighting","version":"v1","selector":null}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn get_services_returns_created_service() {
        let app = build_router(test_state());

        let create_req = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"lighting","version":"v1","selector":null}"#,
            ))
            .unwrap();

        let _ = app.clone().oneshot(create_req).await.unwrap();

        let list_req = Request::builder()
            .method("GET")
            .uri("/services")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(list_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let list: ListServicesResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.services.len(), 1);
        assert_eq!(list.services[0].name, "lighting");
    }

    #[tokio::test]
    async fn post_services_rejects_empty_name() {
        let app = build_router(test_state());

        let req = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"","version":"v1","selector":null}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post_services_returns_conflict_for_duplicate_name_and_version() {
        let app = build_router(test_state());

        let req_body = r#"{"name":"lighting","version":"v1","selector":null}"#;

        let first = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(req_body))
            .unwrap();

        let second = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(req_body))
            .unwrap();

        let resp1 = app.clone().oneshot(first).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::CREATED);

        let resp2 = app.oneshot(second).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn creating_service_triggers_assignment_to_healthy_node() {
        let state = test_state();

        let node_id = NodeId::new();
        {
            let mut guard = state.inner.lock().await;
            guard.nodes.insert(
                node_id,
                NodeRecord {
                    id: node_id,
                    status: NodeStatus::Healthy,
                    last_heartbeat: Some(Utc::now()),
                    capabilities: vec![],
                },
            );
            guard.assignments.insert(node_id, vec![]);
        }

        let app = build_router(state);

        let create_req = Request::builder()
            .method("POST")
            .uri("/services")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"lighting","version":"v1","selector":null}"#,
            ))
            .unwrap();

        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let assignments_req = Request::builder()
            .method("GET")
            .uri(format!("/assignments/{}", node_id))
            .body(Body::empty())
            .unwrap();

        let assignments_resp = app.oneshot(assignments_req).await.unwrap();
        assert_eq!(assignments_resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(assignments_resp.into_body(), usize::MAX)
            .await
            .unwrap();

        let assignments: Vec<homeedge_types::service::ServiceAssignment> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].node_id, node_id);
    }
}
