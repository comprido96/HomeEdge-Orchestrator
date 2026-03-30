// Pointing at ServiceStatus
#[derive(Debug, Clone)]
pub struct ServiceState {
    _placeholder: (),
}

impl ServiceState {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}
