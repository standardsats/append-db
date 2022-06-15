pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

#[derive(Error, Debug)]
/// We can fail either due state update logic or storage backend failure
///
/// Note: we cannot use associated types here as it will require 'Debug' impl for 
/// storages. 
pub enum AppendErr<BackErr, UpdErr> {
    #[error("Update state: {0}")]
    Update(UpdErr),
    #[error("Backend: {0}")]
    Backend(BackErr),
}

pub struct AppendDb<T: StateBackend> {
    pub backend: T,
    pub last_state: Arc<Mutex<T::State>>,
}

impl<St: Clone + State + 'static, Backend: StateBackend<State = St>> AppendDb<Backend> {
    pub fn new(backend: Backend, initial_state: St) -> Self {
        AppendDb {
            backend,
            last_state: Arc::new(Mutex::new(initial_state)),
        }
    }

    pub async fn get(&self) -> MutexGuard<'_, St> {
        self.last_state.lock().await
    }

    pub async fn update(&mut self, upd: St::Update) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let mut state = self.last_state.lock().await;
        self.backend
            .write(SnapshotedUpdate::Incremental(upd.clone()))
            .await
            .map_err(AppendErr::Backend)?;
        state.update(upd).map_err(AppendErr::Update)?;
        Ok(())
    }

    /// Write down snapshot for current state
    pub async fn snapshot(&mut self) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let state = self.last_state.lock().await;
        self.backend
            .write(SnapshotedUpdate::Snapshot(state.deref().clone()))
            .await.map_err(AppendErr::Backend)?;
        Ok(())
    }
}
