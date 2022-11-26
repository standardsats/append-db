pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

/// We can fail either due state update logic or storage backend failure
///
/// Note: we cannot use associated types here as it will require 'Debug' impl for
/// storages.
#[derive(Error, Debug)]
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
    /// Initialize with given backend and strarting in memory state
    pub fn new(backend: Backend, initial_state: St) -> Self {
        AppendDb {
            backend,
            last_state: Arc::new(Mutex::new(initial_state)),
        }
    }

    /// Get current in memory state
    pub async fn get(&self) -> MutexGuard<'_, St> {
        self.last_state.lock().await
    }

    /// Write down to storage new update and update in memory version
    pub async fn update(
        &mut self,
        upd: St::Update,
    ) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let mut state = self.last_state.lock().await;
        state.update(upd.clone()).map_err(AppendErr::Update)?;
        self.backend
            .write(SnapshotedUpdate::Incremental(upd))
            .await
            .map_err(AppendErr::Backend)?;
        Ok(())
    }

    /// Write down snapshot for current state
    pub async fn snapshot(&mut self) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let state = self.last_state.lock().await;
        self.backend
            .write(SnapshotedUpdate::Snapshot(state.deref().clone()))
            .await
            .map_err(AppendErr::Backend)?;
        Ok(())
    }

    /// Load state from storage
    pub async fn load(&mut self) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let updates = self.backend.updates().await.map_err(AppendErr::Backend)?;

        let (mut state, start_index) = match updates.first() {
            Some(SnapshotedUpdate::Snapshot(s)) => (s.clone(), 1),
            _ => (self.last_state.lock().await.deref().clone(), 0),
        };
        
        for upd in &updates[start_index..] {
            match upd {
                SnapshotedUpdate::Snapshot(s) => state = s.clone(),
                SnapshotedUpdate::Incremental(upd) => {
                    state.update(upd.clone()).map_err(AppendErr::Update)?
                }
            }
        }
        let mut cur_state = self.last_state.lock().await;
        *cur_state = state;

        Ok(())
    }
}
