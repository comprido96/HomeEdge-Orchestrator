use async_trait::async_trait;
use chrono::{DateTime, Utc};
use homeedge_types::node::{NodeId, NodeRecord, NodeStatus};

use super::error::RepositoryError;

#[async_trait]
pub trait NodeRepository: Send + Sync {
    async fn upsert(&self, node: &NodeRecord) -> Result<(), RepositoryError>;

    async fn get(&self, node_id: NodeId) -> Result<Option<NodeRecord>, RepositoryError>;

    async fn list(&self) -> Result<Vec<NodeRecord>, RepositoryError>;

    async fn set_heartbeat(
        &self,
        node_id: NodeId,
        timestamp: DateTime<Utc>,
        status: NodeStatus,
    ) -> Result<(), RepositoryError>;

    async fn set_status(
        &self,
        node_id: NodeId,
        status: NodeStatus,
    ) -> Result<(), RepositoryError>;
}

