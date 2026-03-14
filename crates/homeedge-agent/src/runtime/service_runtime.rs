use homeedge_types::ServiceAssignment;

#[derive(Debug)]
pub struct ServiceRuntime {
    _placeholder: (),
}

impl ServiceRuntime {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub async fn start(&self, _assignment: ServiceAssignment) {
        todo!()
    }

    pub async fn stop(&self, _assignment: ServiceAssignment) {
        todo!()
    }
}
