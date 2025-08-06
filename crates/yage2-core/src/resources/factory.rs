use crate::resources::resource::ResourceID;
use crossbeam_queue::ArrayQueue;
use std::ptr::NonNull;

pub enum InMessage {
    Load(ResourceID),
    Free(ResourceID),
}

pub enum OutMessage {
    Loaded(ResourceID, NonNull<()>),
    Freed(ResourceID),
}

pub trait Factory {
    fn new(in_queue: ArrayQueue<InMessage>, out_queue: ArrayQueue<OutMessage>) -> Self
    where
        Self: Sized;
}
