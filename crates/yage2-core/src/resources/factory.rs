use crate::resources::reader::ResourceHeader;
use crate::resources::resource::{ResourceID, ResourceType};
use crossbeam_queue::ArrayQueue;
use std::ptr::NonNull;
use std::sync::Arc;

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
    Load(QueryID, ResourceID, Vec<u8>, ResourceHeader),
    Free(QueryID, ResourceID),
}

pub enum OutMessage {
    Loaded(QueryID, ResourceID, NonNull<()>),
    Freed(QueryID, ResourceID),
}

struct FactoryBindingInner {
    pub resource_type: ResourceType,
    pub in_queue: Arc<ArrayQueue<InMessage>>,
    pub out_queue: Arc<ArrayQueue<OutMessage>>,
}

#[derive(Clone)]
pub struct FactoryBinding(Arc<FactoryBindingInner>);

impl FactoryBinding {
    pub fn new(
        resource_type: ResourceType,
        in_queue: Arc<ArrayQueue<InMessage>>,
        out_queue: Arc<ArrayQueue<OutMessage>>,
    ) -> Self {
        FactoryBinding(Arc::new(FactoryBindingInner {
            resource_type,
            in_queue,
            out_queue,
        }))
    }

    pub fn resource_type(&self) -> ResourceType {
        self.0.resource_type
    }

    pub fn in_queue(&self) -> Arc<ArrayQueue<InMessage>> {
        Arc::clone(&self.0.in_queue)
    }

    pub fn out_queue(&self) -> Arc<ArrayQueue<OutMessage>> {
        Arc::clone(&self.0.out_queue)
    }
}
