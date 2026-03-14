use axum::{
    routing::{get, post},
    Router,
};

use crate::{app_state::AppState, handlers};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/heartbeat", post(handlers::heartbeat))
        .route("/assignments/{node_id}", get(handlers::get_assignments))
        .route("/nodes", get(handlers::list_nodes))
        .route("/services", post(handlers::create_service).get(handlers::list_services),)
        .with_state(state)
}
