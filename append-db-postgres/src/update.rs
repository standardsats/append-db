use append_db::db::{SnapshotedUpdate, State};
use std::borrow::Cow;
use std::fmt;
use thiserror::Error;

/// Update tags are simple strings.
pub type UpdateTag = Cow<'static, str>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct UnknownUpdateTag(pub String);

impl std::error::Error for UnknownUpdateTag {}

impl fmt::Display for UnknownUpdateTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Given UpdateTag '{}' is unknown", self.0)
    }
}

#[derive(Error, Debug)]
pub enum UpdateBodyError {
    #[error("Unknown update tag: {0}")]
    UnknownTag(#[from] UnknownUpdateTag),
    #[error("Failed to deserialize update with version {0} and tag {1}: {2}. Body: {3}")]
    Deserialize(u16, UpdateTag, serde_json::Error, serde_json::Value),
    #[error("Failed to serialize update with tag {0}: {1}")]
    Serialize(UpdateTag, serde_json::Error),
    #[error("Unknown version tag: {0}")]
    UnexpectedVersion(u16),
}

pub trait HasUpdateTag {
    /// Deserialize given JSON tagged value
    /// according to version number.
    fn deserialize_by_tag(
        tag: &UpdateTag,
        version: u16,
        value: serde_json::Value,
    ) -> Result<Self, UpdateBodyError>
    where
        Self: std::marker::Sized;

    /// Get tag of the value. Don't use 'snapshot' tag
    /// as it is internal for snapshots updates.
    fn get_tag(&self) -> UpdateTag;

    /// Get current version of the value
    fn get_version(&self) -> u16;

    /// Serialize internal value without tag. If 'Self'
    /// is enum with variants A(AValue) and B(BValue),
    /// the serialized values should be AValue and BValue
    /// without wrappers.
    fn serialize_untagged(&self) -> Result<serde_json::Value, UpdateBodyError>;
}

/// Helps to deserialized snapshoted states
pub trait VersionedState {
    /// Deserialize snapshoted state with given version tag.
    fn deserialize_with_version(
        version: u16,
        value: serde_json::Value,
    ) -> Result<Self, UpdateBodyError>
    where
        Self: std::marker::Sized;

    /// Get current version of the state
    fn get_version(&self) -> u16;

    /// Serialize current state into JSON value with the current version in mind
    fn serialize(&self) -> Result<serde_json::Value, UpdateBodyError>;
}

pub const SNAPSHOT_TAG: &str = "snapshot";

impl<Upd: HasUpdateTag, St: State<Update = Upd> + VersionedState> HasUpdateTag
    for SnapshotedUpdate<St>
{
    fn deserialize_by_tag(
        tag: &UpdateTag,
        version: u16,
        value: serde_json::Value,
    ) -> Result<Self, UpdateBodyError>
    where
        Self: std::marker::Sized,
    {
        if tag == SNAPSHOT_TAG {
            let st = St::deserialize_with_version(version, value)?;
            Ok(SnapshotedUpdate::Snapshot(st))
        } else {
            let res = Upd::deserialize_by_tag(tag, version, value)?;
            Ok(SnapshotedUpdate::Incremental(res))
        }
    }

    fn get_tag(&self) -> UpdateTag {
        match self {
            SnapshotedUpdate::Snapshot(_) => Cow::Borrowed(SNAPSHOT_TAG),
            SnapshotedUpdate::Incremental(v) => v.get_tag(),
        }
    }

    fn get_version(&self) -> u16 {
        match self {
            SnapshotedUpdate::Snapshot(v) => v.get_version(),
            SnapshotedUpdate::Incremental(v) => v.get_version(),
        }
    }

    fn serialize_untagged(&self) -> Result<serde_json::Value, UpdateBodyError> {
        match self {
            SnapshotedUpdate::Snapshot(v) => v.serialize(),
            SnapshotedUpdate::Incremental(v) => v.serialize_untagged(),
        }
    }
}
