use std::collections::{HashMap, HashSet};
use homeedge_types::service::{ServiceDefinition, ServiceId, ServiceStatus};

use crate::error::AgentError;
use crate::runtime::service_instance::ServiceInstance;
use crate::runtime::worker::run_simulated_service;


#[derive(Debug, Default)]
pub struct ServiceManager {
    instances: HashMap<ServiceId, ServiceInstance>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    pub async fn start(&mut self, svc: &ServiceDefinition) -> Result<(), AgentError> {
        if self.instances.contains_key(&svc.id) {
            tracing::debug!(
                service_id = %svc.id,
                service_name = %svc.name,
                "start skipped; service already managed"
            );
            return Ok(());
        }

        tracing::info!(
            service_id = %svc.id,
            service_name = %svc.name,
            service_version = %svc.version,
            "starting service"
        );

        self.instances
            .insert(svc.id, ServiceInstance::starting(svc.clone()));

        let service_id = svc.id;
        let service_name = svc.name.clone();

        let handle = tokio::spawn(async move {
            run_simulated_service(service_id, service_name).await;
        });

        self.instances.insert(svc.id, ServiceInstance::running(svc.clone(), handle));

        tracing::info!(
            service_id = %svc.id,
            service_name = %svc.name,
            "service started"
        );

        Ok(())
    }

    pub async fn stop(&mut self, id: &ServiceId) -> Result<(), AgentError> {
        let Some(mut instance) = self.instances.remove(id) else {
            tracing::debug!(service_id = %id, "stop skipped; service not managed");
            return Ok(());
        };

        tracing::info!(
            service_id = %id,
            service_name = %instance.definition.name,
            "stopping service"
        );

        if let Some(task) = instance.task.take() {
            task.abort();
        }

        tracing::info!(
            service_id = %id,
            service_name = %instance.definition.name,
            "service stopped"
        );

        Ok(())
    }

    pub fn status(&mut self, id: &ServiceId) -> Option<ServiceStatus> {
        let instance = self.instances.get_mut(id)?;
        instance.refresh_status();
        Some(instance.status)
    }

    pub fn running_ids(&mut self) -> HashSet<ServiceId> {
        self.refresh_all();
        self.instances
            .iter()
            .filter_map(|(id, instance)| {
                (instance.status == ServiceStatus::Running).then_some(*id)
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub async fn stop_all(&mut self) {
        let ids: Vec<ServiceId> = self.instances.keys().copied().collect();

        for id in ids {
            let _ = self.stop(&id).await;
        }
    }

    fn refresh_all(&mut self) {
        for instance in self.instances.values_mut() {
            instance.refresh_status();
        }
    }

    pub fn snapshot_statuses(&mut self) -> Vec<(ServiceId, ServiceStatus)> {
        self.refresh_all();

        let mut statuses: Vec<(ServiceId, ServiceStatus)> = self
            .instances
            .iter()
            .map(|(id, instance)| (*id, instance.status))
            .collect();

        statuses.sort_by_key(|(id, _)| id.0);
        statuses
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use homeedge_types::service::ServiceDefinition;

    fn svc(name: &str) -> ServiceDefinition {
        ServiceDefinition::new(name, "v1")
    }

    #[tokio::test]
    async fn start_registers_running_instance() {
        let mut manager = ServiceManager::new();
        let service = svc("lighting");

        manager.start(&service).await.unwrap();

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.status(&service.id), Some(ServiceStatus::Running));

        manager.stop_all().await;
    }

    #[tokio::test]
    async fn start_is_idempotent_for_same_service() {
        let mut manager = ServiceManager::new();
        let service = svc("lighting");

        manager.start(&service).await.unwrap();
        manager.start(&service).await.unwrap();

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.status(&service.id), Some(ServiceStatus::Running));

        manager.stop_all().await;
    }

    #[tokio::test]
    async fn stop_removes_instance() {
        let mut manager = ServiceManager::new();
        let service = svc("lighting");

        manager.start(&service).await.unwrap();
        manager.stop(&service.id).await.unwrap();

        assert!(manager.is_empty());
        assert_eq!(manager.status(&service.id), None);
    }

    #[tokio::test]
    async fn stop_is_idempotent_for_missing_service() {
        let mut manager = ServiceManager::new();
        let service = svc("lighting");

        manager.stop(&service.id).await.unwrap();

        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn running_ids_returns_all_started_services() {
        let mut manager = ServiceManager::new();
        let lighting = svc("lighting");
        let hvac = svc("hvac");

        manager.start(&lighting).await.unwrap();
        manager.start(&hvac).await.unwrap();

        let running = manager.running_ids();

        assert_eq!(running.len(), 2);
        assert!(running.contains(&lighting.id));
        assert!(running.contains(&hvac.id));

        manager.stop_all().await;
    }

    #[tokio::test]
    async fn stop_all_clears_all_instances() {
        let mut manager = ServiceManager::new();
        manager.start(&svc("lighting")).await.unwrap();
        manager.start(&svc("hvac")).await.unwrap();
        manager.stop_all().await;
        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn refresh_status_marks_finished_task_as_failed() {
        let handle = tokio::spawn(async {});
        handle.await.unwrap_or(());

        // spawn a new already-finished task
        let finished_handle = tokio::spawn(async {});
        tokio::time::sleep(Duration::from_millis(10)).await;

        let mut instance = ServiceInstance::running(svc("lighting"), finished_handle);
        instance.refresh_status();

        assert_eq!(instance.status, ServiceStatus::Failed);
        assert!(instance.task.is_none());
    }

    #[test]
    fn assigned_instance_has_no_task() {
        let instance = ServiceInstance::assigned(svc("lighting"));
        assert_eq!(instance.status, ServiceStatus::Assigned);
        assert!(instance.task.is_none());
    }

    #[tokio::test]
    async fn simulated_worker_can_be_spawned_and_aborted() {
        let id = ServiceId::new();
        let handle = tokio::spawn(run_simulated_service(id, "lighting".to_string()));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!handle.is_finished());

        handle.abort();
    }
}
