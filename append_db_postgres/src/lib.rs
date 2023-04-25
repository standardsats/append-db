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
    use std::convert::Infallible;
    use std::ops::Deref;
    use crate as append_db_postgres;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, VersionedState)]
    struct State0 {
        field: u64,
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, VersionedState)]
    struct State1 {
        field: String
    }

    #[derive(Clone, Debug, PartialEq, HasUpdateTag)]
    enum Update1{
        Append(String),
        Set(String)
    }

    #[derive(Clone, Debug, PartialEq, HasUpdateTag)]
    enum Update0 {
        Add(u64),
        Set(u64),
    }

    impl State for State0 {
        type Update = Update0;
        type Err = Infallible;

        fn update(&mut self, upd: Update0) -> Result<(), Self::Err> {
            match upd {
                Update0::Add(v) => self.field += v,
                Update0::Set(v) => self.field = v,
            }
            Ok(())
        }
    }

    impl State for State1 {
        type Update = Update1;
        type Err = Infallible;

        const TABLE: &'static str = "updates2";

        fn update(&mut self, upd: Self::Update) -> Result<(), Self::Err> {
            match upd {
                Update1::Append(s) => self.field.push_str(s.as_str()),
                Update1::Set(s) => self.field = s,
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
    async fn two_tables_postgres_init() {
        let postgres = Postgres::new(pool);
        let state0 = State0 { field: 42 };
        let db = AppendDb::new(postgres.clone(), state0.clone());
        assert_eq!(db.get().await.deref(), &state0);

        let state1 = State1 {field: String::new()};
        let postgres1 = postgres.duplicate();
        let db = AppendDb::new(postgres1, state1.clone());
        assert_eq!(db.get().await.deref(), &state1);
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
    async fn two_tables_test_updates(){
        let postgres = Postgres::new(pool);
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(postgres.clone(), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 43);
        db.update(Update0::Set(4)).await.expect("update");
        assert_eq!(db.get().await.deref().field, 4);

        let state1 = State1 {field: String::new()};
        let postgres1 = postgres.duplicate();
        let mut db1 = AppendDb::new(postgres1, state1.clone());
        db1.update(Update1::Append("Hello".to_string())).await.expect("update");
        assert_eq!(db1.get().await.deref().field, "Hello".to_string());
        db1.update(Update1::Set("Hello world!".to_string())).await.expect("update");
        assert_eq!(db1.get().await.deref().field, "Hello world!".to_string());        
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
    async fn two_tables_postgres_snapshot() {
        let postgres0 = Postgres::new(pool);
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(postgres0.clone(), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");

        let upds = db.backend.updates().await.expect("collected");
        assert_eq!(upds, vec![SnapshotedUpdate::Snapshot(State0 { field: 43 })]);

        let postgres1 = postgres0.duplicate();
        let state1 = State1 { field: "Hello".to_string() };
        let mut db1 = AppendDb::new(postgres1, state1.clone());
        db1.update(Update1::Append(" world!".to_string())).await.expect("update");
        db1.snapshot().await.expect("snapshot");
        let upds1 = db1.backend.updates().await.expect("collected");
        assert_eq!(upds1, vec![SnapshotedUpdate::Snapshot(State1 { field: "Hello world!".to_string() })]);
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
    async fn two_tables_postgres_reconstruct() {
        let postgres = Postgres::new(pool);
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(postgres.clone(), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.update(Update0::Set(4)).await.expect("update");
        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);

        let state1 = State1 {field: String::new()};
        let postgres1 = postgres.duplicate();
        let mut db1 = AppendDb::new(postgres1, state1.clone());
        db1.update(Update1::Append("Hello".to_string())).await.expect("update");
        db1.update(Update1::Set("Hello world!".to_string())).await.expect("update");

        db1.load().await.expect("load");
        assert_eq!(db1.get().await.deref().field, "Hello world!".to_string());
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

    #[sqlx_database_tester::test(pool(variable = "pool", migrations = "./migrations"))]
    async fn two_tables_postgres_reconstruct_snapshot() {
        let postgres = Postgres::new(pool);
        let state0 = State0 { field: 42 };
        let mut db = AppendDb::new(postgres.clone(), state0.clone());
        db.update(Update0::Add(1)).await.expect("update");
        db.snapshot().await.expect("snapshot");
        db.update(Update0::Set(4)).await.expect("update");

        db.load().await.expect("load");
        assert_eq!(db.get().await.deref().field, 4);

        let state1 = State1 {field: String::new()};
        let postgres1 = postgres.duplicate();
        let mut db1 = AppendDb::new(postgres1, state1.clone());

        db1.update(Update1::Append("Hello".to_string())).await.expect("update");
        db1.snapshot().await.expect("snapshot");
        db1.update(Update1::Set("Hello world!".to_string())).await.expect("update");

        db1.load().await.expect("load");
        assert_eq!(db1.get().await.deref().field, "Hello world!".to_string());
    }
}
