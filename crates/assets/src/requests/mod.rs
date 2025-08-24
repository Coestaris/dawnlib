pub(crate) mod scheduler;
pub mod task;

use crate::{AssetID, AssetType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetRequestID(usize);

impl std::fmt::Display for AssetRequestID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetRequest({})", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum AssetRequestQuery {
    ByID(AssetID),
    ByTag(String),
    ByTags(Vec<String>),
    ByType(AssetType),
    All,
}

#[derive(Debug, Clone)]
pub enum AssetRequest {
    Enumerate,
    Read(AssetRequestQuery),
    ReadNoDeps(AssetRequestQuery),
    Load(AssetRequestQuery),
    LoadNoDeps(AssetRequestQuery),
    Free(AssetRequestQuery),
    FreeNoDeps(AssetRequestQuery),
}

impl Default for AssetRequestID {
    fn default() -> Self {
        AssetRequestID(0)
    }
}

impl AssetRequestID {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AssetRequestID(id)
    }
}
