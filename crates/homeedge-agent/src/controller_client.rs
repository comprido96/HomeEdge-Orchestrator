use homeedge_types::{HeartbeatRequest, RegisterRequest, ServiceAssignment};

#[derive(Debug)]
pub struct ClientError;

#[derive(Debug, Clone)]
pub struct ControllerClient {
    // Sprint 0 placeholder
    _placeholder: (),
}

impl ControllerClient {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub async fn register(&self, _request: RegisterRequest) -> Result<(), ClientError> {
        todo!()
    }

    pub async fn heartbeat(&self, _request: HeartbeatRequest) -> Result<(), ClientError> {
        todo!()
    }

    pub async fn fetch_assignments(&self) -> Result<Vec<ServiceAssignment>, ClientError> {
        todo!()
    }
}
