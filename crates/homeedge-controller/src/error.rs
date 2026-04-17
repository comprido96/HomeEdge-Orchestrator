use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

use crate::repository::RepositoryError;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("node not found")]
    NodeNotFound,

    #[error("service not found")]
    ServiceNotFound,

    #[error("invalid request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NodeNotFound | Self::ServiceNotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) | Self::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::NodeNotFound => "node_not_found",
            Self::ServiceNotFound => "service_not_found",
            Self::BadRequest(_) => "bad_request",
            Self::Conflict(_) => "conflict",
            Self::Internal(_) => "internal_error",
            Self::Repository(_) => "repository_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        let body = Json(ErrorResponse {
            error: String::from(self.kind()),
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}


#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    use super::AppError;

    async fn response_parts(err: AppError) -> (StatusCode, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn node_not_found_error_serializes_correctly() {
        let (status, json) = response_parts(AppError::NodeNotFound).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"], "node_not_found");
        assert_eq!(json["message"], "node not found");
    }

    #[tokio::test]
    async fn bad_request_error_serializes_correctly() {
        let (status, json) = response_parts(
            AppError::BadRequest("name must not be empty".to_string())
        ).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "bad_request");
        assert!(json["message"].as_str().unwrap().contains("name must not be empty"));
    }

    #[tokio::test]
    async fn internal_error_serializes_correctly() {
        let (status, json) = response_parts(
            AppError::Internal("unexpected state".to_string())
        ).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["error"], "internal_error");
    }
}
