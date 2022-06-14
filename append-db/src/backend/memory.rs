pub use crate::backend::class::StateBackend;

pub struct InMemory<T> {
    pub state: T,
}

impl<T> InMemory<T> {
    pub fn new(init_state: T) -> Self {
        InMemory {
            state: init_state,
        }
    }
}

impl<T> StateBackend for InMemory<T> {
    type State = T;
}