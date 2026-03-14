use axum::{
    routing::{get, post},
    Router,
};

use crate::{app_state::AppState, handlers};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/register", post(handlers::register))
        .route("/heartbeat", post(handlers::heartbeat))
        .route("/assignments/{node_id}", get(handlers::assignments))
        .route("/nodes", get(handlers::nodes))
        .with_state(state)
}
