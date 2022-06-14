pub use crate::backend::class::{State, StateBackend};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

pub struct AppendDb<T: StateBackend> {
    pub backend: T,
    pub last_state: Arc<Mutex<T::State>>,
}

impl<St: State + 'static, Backend: StateBackend<State = St>> AppendDb<Backend> {
    pub fn new(backend: Backend, initial_state: St) -> Self {
        AppendDb {
            backend,
            last_state: Arc::new(Mutex::new(initial_state)),
        }
    }

    pub async fn get(&self) -> MutexGuard<'_, St> {
        self.last_state.lock().await
    }

    pub async fn update(&mut self, upd: St::Update) {
        let mut state = self.last_state.lock().await;
        self.backend.write(upd.clone()).await;
        state.update(upd);
    }
}
