pub mod backend;
pub mod db;

pub use backend::class::*;

#[cfg(test)]
mod tests {
    use super::backend::class::{SnapshotedUpdate, State, StateBackend};
    use super::backend::memory::InMemory;
    use super::db::AppendDb;
    use std::convert::Infallible;
    use std::ops::Deref;

    #[derive(Clone, Debug, PartialEq)]
    struct State0 {
        field: u64,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Update0 {
        Add(u64),
        Set(u64),
    }

    impl State for State0 {
        type Update = Update0;
        type Err = Infallible;
        const TABLE: &'static str = "updates";

        fn update(&mut self, upd: Update0) -> Result<(), Self::Err> {
            match upd {
                Update0::Add(v) => self.field += v,
                Update0::Set(v) => self.field = v,
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn in_memory_init() {
        let state0 = State0 { field: 42 };
        let db = AppendDb::new(InMemory::new(), state0.clone());
        assert_eq!(db.get().await.deref(), &state0);
    }

    #[tokio::test]
    async fn in_memory_updates() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(InMemory::new(), state0);
        db.update(Update0::Add(1)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 43);
        db.update(Update0::Set(4)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 4);
    }

    #[tokio::test]
    async fn in_memory_snapshot() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(InMemory::new(), state0);
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");

        let upds = db.backend.updates().await.expect("collected");
        assert_eq!(upds, vec![SnapshotedUpdate::Snapshot(State0 { field: 43 })])
    }

    #[tokio::test]
    async fn in_memory_reconstruct() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(InMemory::new(), state0);
        db.update(Update0::Add(1)).await.expect("update");
        db.update(Update0::Set(4)).await.expect("update");

        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);
    }

    #[tokio::test]
    async fn in_memory_reconstruct_snapshot() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(InMemory::new(), state0);
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");
        db.update(Update0::Set(4)).await.expect("update");

        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);
    }
}
