use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use homeedge_controller::app_state::{AppState, StorageMode};
use homeedge_controller::router::build_router;
use homeedge_types::{ServiceAssignment, api::{
    AssignmentsResponse, HeartbeatRequest, HeartbeatResponse, NodesResponse, RegisterRequest,
    RegisterResponse,
}};
use homeedge_types::node::{NodeId, NodeStatus};

fn node_id(n: u128) -> NodeId {
    NodeId(Uuid::from_u128(n))
}

#[tokio::test]
async fn post_register_creates_node() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state);

    let req_body = serde_json::to_vec(&RegisterRequest {
        node_id: node_id(1),
        capabilities: vec!["docker".into()],
    })
    .unwrap();

    let response = app
        .oneshot(
            Request::post("/register")
                .header("content-type", "application/json")
                .body(Body::from(req_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: RegisterResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(parsed.node.id, node_id(1));
    assert_eq!(parsed.node.status, NodeStatus::Registering);
    assert_eq!(parsed.node.capabilities, vec!["docker"]);
}

#[tokio::test]
async fn post_heartbeat_marks_registered_node_healthy() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state.clone());

    let register_body = serde_json::to_vec(&RegisterRequest {
        node_id: node_id(2),
        capabilities: vec!["mqtt".into()],
    })
    .unwrap();

    app.clone()
        .oneshot(
            Request::post("/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let ts = Utc::now();

    let heartbeat_body = serde_json::to_vec(&HeartbeatRequest {
        node_id: node_id(2),
        timestamp: ts,
        service_statuses: vec![],
    })
    .unwrap();

    let response = app
        .oneshot(
            Request::post("/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(heartbeat_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: HeartbeatResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(parsed.node.id, node_id(2));
    assert_eq!(parsed.node.status, NodeStatus::Healthy);
    assert_eq!(parsed.node.last_heartbeat, Some(ts));
}

#[tokio::test]
async fn post_heartbeat_for_unknown_node_returns_404() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state);

    let heartbeat_body = serde_json::to_vec(&HeartbeatRequest {
        node_id: node_id(999),
        timestamp: Utc::now(),
        service_statuses: vec![],
    })
    .unwrap();

    let response = app
        .oneshot(
            Request::post("/heartbeat")
                .header("content-type", "application/json")
                .body(Body::from(heartbeat_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_assignments_for_registered_node_returns_empty_list() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state.clone());

    let register_body = serde_json::to_vec(&RegisterRequest {
        node_id: node_id(3),
        capabilities: vec![],
    })
    .unwrap();

    app.clone()
        .oneshot(
            Request::post("/register")
                .header("content-type", "application/json")
                .body(Body::from(register_body))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::get("/assignments/00000000-0000-0000-0000-000000000003")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: Vec<ServiceAssignment> = serde_json::from_slice(&body).unwrap();

    assert!(parsed.is_empty());
}

#[tokio::test]
async fn get_assignments_for_unknown_node_returns_404() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::get("/assignments/00000000-0000-0000-0000-000000000099")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_nodes_returns_registered_nodes() {
    let state = AppState::new(StorageMode::InMemory);
    let app = build_router(state.clone());

    for (id, capability) in [(10_u128, "docker"), (20_u128, "mqtt")] {
        let register_body = serde_json::to_vec(&RegisterRequest {
            node_id: node_id(id),
            capabilities: vec![capability.into()],
        })
        .unwrap();

        app.clone()
            .oneshot(
                Request::post("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(register_body))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let response = app
        .oneshot(Request::get("/nodes").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let parsed: NodesResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(parsed.nodes.len(), 2);
    assert_eq!(parsed.nodes[0].node.id, node_id(10));
    assert_eq!(parsed.nodes[1].node.id, node_id(20));
}
