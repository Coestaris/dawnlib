use crate::requests::AssetRequestID;
use crate::AssetID;

#[derive(Debug, Clone)]
pub(crate) enum TaskCommand {
    Enumerate,
    Read(AssetID),
    Load(AssetID),
    Free(AssetID),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetTaskID(AssetRequestID, usize);

impl AssetTaskID {
    pub fn new(qid: AssetRequestID) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AssetTaskID(qid, id)
    }

    pub fn as_request(&self) -> AssetRequestID {
        self.0
    }

    pub fn as_task_id(&self) -> usize {
        self.1
    }
}

impl std::fmt::Display for AssetTaskID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskID({}, {})", self.0, self.1)
    }
}
