pub mod backend;
pub mod db;

#[cfg(test)]
mod tests {
    use super::db::AppendDb;
    use super::backend::class::State;
    use super::backend::memory::InMemory;
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

        fn update(&mut self, upd: Update0) {
            match upd {
                Update0::Add(v) => self.field += v,
                Update0::Set(v) => self.field = v,
            }
        }
    }

    #[tokio::test]
    async fn in_memory_init() {
        let state0 = State0 {
            field: 42,
        };
        let db = AppendDb::new(InMemory::new(state0.clone()), state0.clone());
        assert_eq!(db.get().await.deref(), &state0);
    }

    #[tokio::test]
    async fn in_memory_updates() {
        let state0 = State0 {
            field: 42,
        };
        let mut db = AppendDb::new(InMemory::new(state0.clone()), state0.clone());
        db.update(Update0::Add(1)).await;
        assert_eq!(db.get().await.deref().field, 43);
        db.update(Update0::Set(4)).await;
        assert_eq!(db.get().await.deref().field, 4);
    }
}
