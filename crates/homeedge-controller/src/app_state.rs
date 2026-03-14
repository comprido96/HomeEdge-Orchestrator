use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Debug, Default)]
struct AppStateInner {
    // Sprint 0 placeholder.
    // Replace with real controller state in Sprint 1.
    _placeholder: (),
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AppStateInner { _placeholder: () }),
        }
    }

    // pub fn inner(&self) -> &Arc<AppStateInner> {
    //     &self.inner
    // }
}
