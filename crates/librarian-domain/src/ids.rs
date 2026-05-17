//! Newtype identifiers. Each wraps a `String` so the compiler refuses to mix
//! e.g. a `SourceId` with a `ChunkId` at call sites.

use serde::{Deserialize, Serialize};

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);
    };
}

string_newtype!(SourceId);
string_newtype!(WorkId);
string_newtype!(SnapshotId);
string_newtype!(SourceHash);
string_newtype!(ConfigHash);
string_newtype!(CacheKey);
string_newtype!(StageVersion);
string_newtype!(ChunkId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtype_equality_is_value_based() {
        assert_eq!(SourceId("a".into()), SourceId("a".into()));
        assert_ne!(SourceId("a".into()), SourceId("b".into()));
    }
}
