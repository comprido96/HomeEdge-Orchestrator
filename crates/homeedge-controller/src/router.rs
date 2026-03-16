use axum::{
    Router, routing::{get, post, put}
};

use crate::{app_state::AppState, handlers};

pub fn build_router(state: AppState) -> Router {
    Router::new()
    .route("/register", post(handlers::register))
    .route("/heartbeat", post(handlers::heartbeat))
    .route("/nodes", get(handlers::list_nodes))
    .route("/services",
        post(handlers::create_service)
        .get(handlers::list_services)
    )
    .route("/services/{service_id}",
        get(handlers::get_service)
        .put(handlers::update_service)
        .delete(handlers::delete_service)
    )
    .route("/assignments",
        get(handlers::list_assignments)
    )
    .route("/assignments/{node_id}",
        get(handlers::get_assignments)
    )
    .route("/assignments/service/{service_id}",
        put(handlers::assign_service)
        .delete(handlers::unassign_service)
    )
    .with_state(state)
}
