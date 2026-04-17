use async_trait::async_trait;
use homeedge_types::node::NodeId;
use homeedge_types::service::{ServiceAssignment, ServiceId};

use super::error::RepositoryError;

#[async_trait]
pub trait AssignmentRepository: Send + Sync {
    async fn assign(
        &self,
        service_id: ServiceId,
        node_id: NodeId,
    ) -> Result<(), RepositoryError>;

    async fn unassign(
        &self,
        service_id: ServiceId,
    ) -> Result<(), RepositoryError>;

    async fn for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceAssignment>, RepositoryError>;

    async fn list(&self) -> Result<Vec<ServiceAssignment>, RepositoryError>;

    async fn remove_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceId>, RepositoryError>;
}
