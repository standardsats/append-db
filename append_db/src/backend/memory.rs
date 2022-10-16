pub use crate::backend::class::{SnapshotedUpdate, State, StateBackend};
use async_trait::async_trait;
use std::{sync::Arc, convert::Infallible};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct InMemory<St: State> {
    pub updates: Arc<Mutex<Vec<SnapshotedUpdate<St>>>>,
}

impl<St: State> InMemory<St> {
    pub fn new() -> Self {
        InMemory {
            updates: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl<St: State> Default for InMemory<St> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<St: Clone + State + 'static + Send> StateBackend for InMemory<St> {
    type State = St;
    type Err = Infallible;

    async fn write(&mut self, upd: SnapshotedUpdate<Self::State>) -> Result<(), Self::Err> {
        self.updates.lock().await.push(upd);
        Ok(())
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
