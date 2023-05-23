pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use std::marker::Sync;
use stm::{atomically, TVar};
use thiserror::Error;

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
    pub last_state: TVar<T::State>,
}

impl<St: Clone + State + Sync + Send + 'static, Backend: StateBackend<State = St>>
    AppendDb<Backend>
{
    /// Initialize with given backend and strarting in memory state
    pub fn new(backend: Backend, initial_state: St) -> Self {
        AppendDb {
            backend,
            last_state: TVar::new(initial_state),
        }
    }

    /// Access current state
    pub fn get(&self) -> St {
        atomically(|trans| self.last_state.read(trans))
    }

    /// Write down to storage new update and update in memory version
    pub async fn update(&self, upd: St::Update) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        atomically(|trans| {
            let mut state = self.last_state.read(trans)?;
            let upd = state.update(upd.clone()).map_err(AppendErr::Update);
            match upd {
                Ok(_) => {
                    self.last_state.write(trans, state)?;
                    Ok(Ok(()))
                }
                Err(e) => Ok(Err(e)),
            }
        })?;
        self.backend
            .write(SnapshotedUpdate::Incremental(upd))
            .await
            .map_err(AppendErr::Backend)?;
        Ok(())
    }

    /// Write down snapshot for current state
    pub async fn snapshot(&self) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let state = atomically(|trans| self.last_state.read(trans));
        self.backend
            .write(SnapshotedUpdate::Snapshot(state))
            .await
            .map_err(AppendErr::Backend)?;
        Ok(())
    }

    /// Load state from storage
    pub async fn load(&self) -> Result<(), AppendErr<Backend::Err, St::Err>> {
        let updates = self.backend.updates().await.map_err(AppendErr::Backend)?;

        let (mut state, start_index) = match updates.first() {
            Some(SnapshotedUpdate::Snapshot(s)) => (s.clone(), 1),
            _ => {
                let state = atomically(|trans| self.last_state.read(trans));
                (state, 0)
            }
        };

        for upd in &updates[start_index..] {
            match upd {
                SnapshotedUpdate::Snapshot(s) => state = s.clone(),
                SnapshotedUpdate::Incremental(upd) => {
                    state.update(upd.clone()).map_err(AppendErr::Update)?
                }
            }
        }
        atomically(|trans| self.last_state.write(trans, state.clone()));

        Ok(())
    }

    /// Load state from storage using provided function to patch starting and snapshot states. That
    /// is helpful if you add some runtime info into state that is not rendered in updates.
    ///
    /// The second parameter in the closure indicates whether the patching occurs at start or not.
    /// For later snapshots it will be called with false.
    pub async fn load_patched<F>(
        &self,
        patch_state: F,
    ) -> Result<(), AppendErr<Backend::Err, St::Err>>
    where
        F: Copy + FnOnce(St, bool) -> St,
    {
        let updates = self.backend.updates().await.map_err(AppendErr::Backend)?;

        let (mut state, start_index) = match updates.first() {
            Some(SnapshotedUpdate::Snapshot(s)) => (patch_state(s.clone(), true), 1),
            _ => {
                let state = atomically(|trans| self.last_state.read(trans));
                (patch_state(state, true), 0)
            }
        };

        for upd in &updates[start_index..] {
            match upd {
                SnapshotedUpdate::Snapshot(s) => state = patch_state(s.clone(), false),
                SnapshotedUpdate::Incremental(upd) => {
                    state.update(upd.clone()).map_err(AppendErr::Update)?
                }
            }
        }
        atomically(|trans| self.last_state.write(trans, state.clone()));

        Ok(())
    }
}
