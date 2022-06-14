pub use crate::backend::class::StateBackend;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

pub struct AppendDb<T: StateBackend> {
    pub backend: T,
    pub last_state: Arc<Mutex<T::State>>,
}

impl<T: StateBackend> AppendDb<T> {
    pub fn new(backend: T, initial_state: T::State) -> Self {
        AppendDb {
            backend,
            last_state: Arc::new(Mutex::new(initial_state)),
        }
    }

    pub async fn get(&self) -> MutexGuard<'_, T::State> {
        self.last_state.lock().await
    }
}
