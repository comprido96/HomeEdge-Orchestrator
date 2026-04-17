use async_trait::async_trait;
use homeedge_types::service::{ServiceDefinition, ServiceId};

use super::error::RepositoryError;

#[async_trait]
pub trait ServiceRepository: Send + Sync {
    async fn insert(&self, service: &ServiceDefinition) -> Result<(), RepositoryError>;

    async fn update(&self, service: &ServiceDefinition) -> Result<(), RepositoryError>;

    async fn get(&self, service_id: ServiceId) -> Result<Option<ServiceDefinition>, RepositoryError>;

    async fn list(&self) -> Result<Vec<ServiceDefinition>, RepositoryError>;

    async fn delete(&self, service_id: ServiceId) -> Result<(), RepositoryError>;
}
