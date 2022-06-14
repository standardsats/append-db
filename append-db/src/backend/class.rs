/// Describes a storing backend that can 
/// save and load given internal type of updates for 
/// state.
pub trait StateBackend {
    type State;

}