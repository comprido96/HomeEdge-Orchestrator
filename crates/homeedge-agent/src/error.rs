use reqwest::StatusCode;
use thiserror::Error;
use homeedge_types::{ServiceId, node::NodeId};


#[derive(Debug, Error)]
pub enum AgentError {
    #[error("http request failed during {operation}: {source}")]
    Http {
        operation: &'static str,
        #[source]
        source: reqwest::Error,
    },

    #[error("controller returned {status} during {operation}: {message}")]
    ControllerApi {
        operation: &'static str,
        status: StatusCode,
        message: String,
    },

    #[error("node is not registered: {0}")]
    NodeNotRegistered(NodeId),

    #[error("service definition missing for assigned service {0}")]
    MissingServiceDefinition(ServiceId),

    #[error("service runtime failure: {0}")]
    Runtime(String),
}
