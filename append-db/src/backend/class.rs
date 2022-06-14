use async_trait::async_trait;

/// Describes a storing backend that can 
/// save and load given internal type of updates for 
/// state.
#[async_trait]
pub trait StateBackend {
    /// Aggregated state in memory
    type State: State + 'static;

    /// Write down state update into storage
    async fn write(&mut self, upd: <Self::State as State>::Update);
}

/// Aggregated state that could be updated by small updates
pub trait State {
    /// Incremental single update of the state
    type Update: Clone + Send + 'static; 

    /// Update the state with incremental part
    fn update(&mut self, upd: Self::Update);
}

/// Update with added shapshot to capture points when 
/// we want to save whole state.
pub enum SnapshotedUpdate<St: State> {
    Incremental(St::Update),
    Snapshot(St),
}