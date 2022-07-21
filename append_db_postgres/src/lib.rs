#![feature(never_type)]
pub mod backend;
pub mod update;

pub use update::{HasUpdateTag, VersionedState};
#[cfg(feature = "derive")]
pub use append_db_postgres_derive::*;

#[cfg(test)]
mod tests {
    use crate::backend::Postgres;
    use crate::update::{
        HasUpdateTag, VersionedState,
    };
    use append_db::backend::class::{SnapshotedUpdate, State, StateBackend};
    use append_db::db::AppendDb;
    use append_db_postgres_derive::*;
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;
    use crate as append_db_postgres;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, VersionedState)]
    struct State0 {
        field: u64,
    }

    #[derive(Clone, Debug, PartialEq, HasUpdateTag)]
    enum Update0 {
        Add(u64),
        Set(u64),
    }

    impl State for State0 {
        type Update = Update0;
        type Err = !;

        fn update(&mut self, upd: Update0) -> Result<(), Self::Err> {
            match upd {
                Update0::Add(v) => self.field += v,
                Update0::Set(v) => self.field = v,
            }
            Ok(())
        }
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn postgres_init() {
        let state0 = State0 { field: 42 };
        let db = AppendDb::new(Postgres::new(pool), state0.clone());
        assert_eq!(db.get().await.deref(), &state0);
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn postgres_updates() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(Postgres::new(pool), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 43);
        db.update(Update0::Set(4)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 4);
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn postgres_snapshot() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(Postgres::new(pool), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");

        let upds = db.backend.updates().await.expect("collected");
        assert_eq!(upds, vec![SnapshotedUpdate::Snapshot(State0 { field: 43 })])
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn postgres_reconstruct() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(Postgres::new(pool), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.update(Update0::Set(4)).await.expect("update");

        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);
    }

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn postgres_reconstruct_snapshot() {
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(Postgres::new(pool), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");
        db.update(Update0::Set(4)).await.expect("update");

        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);
    }
}
