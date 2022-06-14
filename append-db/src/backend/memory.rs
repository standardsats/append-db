pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct InMemory<St: State> {
    pub updates: Arc<Mutex<Vec<SnapshotedUpdate<St>>>>,
}

impl<St: State> InMemory<St> {
    pub fn new(init_state: St) -> Self {
        InMemory {
            updates: Arc::new(Mutex::new(vec![SnapshotedUpdate::Snapshot(init_state)])),
        }
    }
}

#[async_trait]
impl<St: State + 'static + Send> StateBackend for InMemory<St> {
    type State = St;

    async fn write(&mut self, upd: <Self::State as State>::Update) {
        self.updates
            .lock()
            .await
            .push(SnapshotedUpdate::Incremental(upd))
    }
}
