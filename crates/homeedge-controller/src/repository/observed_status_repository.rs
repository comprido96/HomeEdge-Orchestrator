use async_trait::async_trait;
use homeedge_types::node::NodeId;
use homeedge_types::service::{ServiceHealthReport, ServiceId, ServiceStatus};

use super::error::RepositoryError;

#[async_trait]
pub trait ObservedStatusRepository: Send + Sync {
    async fn replace_for_node(
        &self,
        node_id: NodeId,
        statuses: &[ServiceHealthReport],
    ) -> Result<(), RepositoryError>;

    async fn list_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceHealthReport>, RepositoryError>;

    async fn clear_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<(), RepositoryError>;
}
