#![feature(never_type)]
pub mod backend;
pub mod update;

#[cfg(test)]
mod tests {
    use crate::backend::Postgres;
    use crate::update::{
        HasUpdateTag, UnknownUpdateTag, UpdateBodyError, UpdateTag, VersionedState, SNAPSHOT_TAG,
    };
    use append_db::backend::class::{SnapshotedUpdate, State, StateBackend};
    use append_db::db::AppendDb;
    use serde::{Deserialize, Serialize};
    use std::borrow::Cow;
    use std::ops::Deref;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct State0 {
        field: u64,
    }

    impl VersionedState for State0 {
        fn deserialize_with_version(
            version: u16,
            value: serde_json::Value,
        ) -> Result<Self, UpdateBodyError> {
            serde_json::from_value(value.clone()).map_err(|e| {
                UpdateBodyError::Deserialize(version, Cow::Borrowed(SNAPSHOT_TAG), e, value)
            })
        }

        fn get_version(&self) -> u16 {
            0
        }

        fn serialize(&self) -> Result<serde_json::Value, UpdateBodyError> {
            Ok(serde_json::to_value(&self)
                .map_err(|e| UpdateBodyError::Serialize(Cow::Borrowed(SNAPSHOT_TAG), e))?)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Update0 {
        Add(u64),
        Set(u64),
    }

    impl HasUpdateTag for Update0 {
        fn deserialize_by_tag(
            tag: &UpdateTag,
            version: u16,
            value: serde_json::Value,
        ) -> Result<Self, UpdateBodyError>
        where
            Self: std::marker::Sized,
        {
            if tag == "add" {
                Ok(Update0::Add(
                    serde_json::from_value(value.clone()).map_err(|e| {
                        UpdateBodyError::Deserialize(version, tag.to_owned(), e, value)
                    })?,
                ))
            } else if tag == "set" {
                Ok(Update0::Set(
                    serde_json::from_value(value.clone()).map_err(|e| {
                        UpdateBodyError::Deserialize(version, tag.to_owned(), e, value)
                    })?,
                ))
            } else {
                Err(UpdateBodyError::UnknownTag(UnknownUpdateTag(
                    tag.to_string(),
                )))
            }
        }

        fn get_tag(&self) -> UpdateTag {
            match self {
                Update0::Add(_) => Cow::Borrowed("add"),
                Update0::Set(_) => Cow::Borrowed("set"),
            }
        }

        fn get_version(&self) -> u16 {
            0
        }

        fn serialize_untagged(&self) -> Result<serde_json::Value, UpdateBodyError> {
            match self {
                Update0::Add(v) => Ok(serde_json::to_value(&v)
                    .map_err(|e| UpdateBodyError::Serialize(self.get_tag(), e))?),
                Update0::Set(v) => Ok(serde_json::to_value(&v)
                    .map_err(|e| UpdateBodyError::Serialize(self.get_tag(), e))?),
            }
        }
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
