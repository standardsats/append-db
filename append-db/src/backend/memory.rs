pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
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
impl<St: Clone + State + 'static + Send> StateBackend for InMemory<St> {
    type State = St;
    type Err = !;

    async fn write(&mut self, upd: SnapshotedUpdate<Self::State>) -> Result<(), Self::Err> {
        Ok(self.updates.lock().await.push(upd))
    }

    async fn updates(&self) -> Result<Vec<SnapshotedUpdate<Self::State>>, Self::Err> {
        let mut res = vec![];

        for v in self.updates.lock().await.iter().rev() {
            res.push(v.clone());
            if v.is_snapshot() {
                break;
            }
        }
        res.reverse();
        Ok(res)
    }
}
