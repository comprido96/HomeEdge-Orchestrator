use tokio::task::JoinHandle;

use homeedge_types::service::{ServiceDefinition, ServiceStatus};

#[derive(Debug)]
pub struct ServiceInstance {
    pub definition: ServiceDefinition,
    // Sprint 3: always Running after spawn; Failed state added in Sprint 4
    pub status: ServiceStatus,
    pub task: Option<JoinHandle<()>>,
}

impl ServiceInstance {
    pub fn assigned(definition: ServiceDefinition) -> Self {
        Self {
            definition,
            status: ServiceStatus::Assigned,
            task: None,
        }
    }

    pub fn starting(definition: ServiceDefinition) -> Self {
        Self {
            definition,
            status: ServiceStatus::Starting,
            task: None,
        }
    }

    pub fn running(definition: ServiceDefinition, task: JoinHandle<()>) -> Self {
        Self {
            definition,
            status: ServiceStatus::Running,
            task: Some(task),
        }
    }

    pub fn failed(definition: ServiceDefinition) -> Self {
        Self {
            definition,
            status: ServiceStatus::Failed,
            task: None,
        }
    }

    pub fn refresh_status(&mut self) {
        if self.status == ServiceStatus::Running {
            if let Some(task) = &self.task {
                if task.is_finished() {
                    self.status = ServiceStatus::Failed;
                    self.task = None;
                }
            }
        }
    }
}
