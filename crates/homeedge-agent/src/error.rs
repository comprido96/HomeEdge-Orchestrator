use reqwest::StatusCode;
use thiserror::Error;

use homeedge_types::node::NodeId;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("controller returned {status}: {message}")]
    ControllerApi {
        status: StatusCode,
        message: String,
    },

    #[error("node {0} is not registered with controller")]
    NodeNotRegistered(NodeId),
}
