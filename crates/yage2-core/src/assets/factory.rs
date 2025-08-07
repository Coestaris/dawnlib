use crossbeam_queue::ArrayQueue;
use std::ptr::NonNull;
use std::sync::Arc;
use crate::assets::reader::AssetHeader;
use crate::assets::{AssetID, AssetType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryID(usize);

impl std::fmt::Display for QueryID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QueryID({})", self.0)
    }
}

impl QueryID {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        QueryID(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

pub enum InMessage {
    Load(QueryID, AssetID, Vec<u8>, AssetHeader),
    Free(QueryID, AssetID),
}

pub enum OutMessage {
    Loaded(QueryID, AssetID, NonNull<()>),
    Freed(QueryID, AssetID),
}

struct FactoryBindingInner {
    pub asset_type: AssetType,
    pub in_queue: Arc<ArrayQueue<InMessage>>,
    pub out_queue: Arc<ArrayQueue<OutMessage>>,
}

#[derive(Clone)]
pub struct FactoryBinding(Arc<FactoryBindingInner>);

impl FactoryBinding {
    pub fn new(
        asset_type: AssetType,
        in_queue: Arc<ArrayQueue<InMessage>>,
        out_queue: Arc<ArrayQueue<OutMessage>>,
    ) -> Self {
        FactoryBinding(Arc::new(FactoryBindingInner {
            asset_type,
            in_queue,
            out_queue,
        }))
    }

    pub fn asset_type(&self) -> AssetType {
        self.0.asset_type
    }

    pub fn in_queue(&self) -> Arc<ArrayQueue<InMessage>> {
        Arc::clone(&self.0.in_queue)
    }

    pub fn out_queue(&self) -> Arc<ArrayQueue<OutMessage>> {
        Arc::clone(&self.0.out_queue)
    }
}
