pub mod backend;
pub mod db;

#[cfg(test)]
mod tests {
    use super::db::AppendDb;
    use super::backend::memory::InMemory;
    use std::ops::Deref;

    #[derive(Clone, Debug, PartialEq)]
    struct State0 {
        field: u64,
    }

    async fn in_memory_init() {
        let state0 = State0 {
            field: 42,
        };
        let db = AppendDb::new(InMemory::new(state0.clone()), state0.clone());
        assert_eq!(db.get().await.deref(), &state0);
    }
}
