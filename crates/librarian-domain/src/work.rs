//! A `Work` is a metadata-only grouping of one or more source documents
//! (e.g. a multi-PDF book whose chapters are separate files).

use serde::{Deserialize, Serialize};

use crate::ids::{SourceId, WorkId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub work_id: WorkId,
    pub title: String,
    pub members: Vec<SourceId>,
}
