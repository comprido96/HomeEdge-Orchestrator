use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("entity not found: {0}")]
    NotFound(&'static str),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("storage error: {0}")]
    Storage(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(value: sqlx::Error) -> Self {
        Self::Storage(value.to_string())
    }
}

impl From<serde_json::Error> for RepositoryError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value.to_string())
    }
}
