use crate::ir::IRAsset;
use crate::{Asset, AssetHeader, AssetID, AssetMemoryUsage, AssetType};
use crossbeam_queue::ArrayQueue;
use log::{error, warn};
use std::any::TypeId;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Arc;
use crate::requests::AssetTaskID;

#[derive(Debug)]
pub struct LoadFactoryMessage {
    pub task_id: AssetTaskID,
    pub asset_id: AssetID,
    pub asset_header: AssetHeader,
    pub ir: IRAsset,
    pub dependencies: HashMap<AssetID, Asset>,
}

#[derive(Debug)]
pub struct FreeFactoryMessage {
    pub task_id: AssetTaskID,
    pub asset_id: AssetID,
}

#[derive(Debug)]
pub enum ToFactoryMessage {
    Load(LoadFactoryMessage),
    Free(FreeFactoryMessage),
}

#[derive(Debug)]
pub struct LoadedFactoryMessage {
    pub task_id: AssetTaskID,
    pub asset_id: AssetID,
    pub usage: AssetMemoryUsage,
    pub asset_type: TypeId,
    pub asset_ptr: NonNull<()>,
}

#[derive(Debug)]
pub struct LoadFailedFactoryMessage {
    pub task_id: AssetTaskID,
    pub asset_id: AssetID,
    pub error: String,
}

#[derive(Debug)]
pub struct FreedFactoryMessage {
    pub task_id: AssetTaskID,
    pub asset_id: AssetID,
}

#[derive(Debug)]
pub enum FromFactoryMessage {
    Loaded(LoadedFactoryMessage),
    LoadFailed(LoadFailedFactoryMessage),
    Freed(FreedFactoryMessage),
}

// Make rust happy with sending NonNull
unsafe impl Send for FromFactoryMessage {}
unsafe impl Sync for FromFactoryMessage {}
unsafe impl Send for ToFactoryMessage {}
unsafe impl Sync for ToFactoryMessage {}

struct FactoryBindingInner {
    pub asset_type: AssetType,
    pub in_queue: Arc<ArrayQueue<ToFactoryMessage>>,
    pub out_queue: Arc<ArrayQueue<FromFactoryMessage>>,
}

#[derive(Clone)]
pub struct FactoryBinding(Arc<FactoryBindingInner>);

impl FactoryBinding {
    pub fn new(
        asset_type: AssetType,
        in_queue: Arc<ArrayQueue<ToFactoryMessage>>,
        out_queue: Arc<ArrayQueue<FromFactoryMessage>>,
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

    pub fn in_queue(&self) -> Arc<ArrayQueue<ToFactoryMessage>> {
        Arc::clone(&self.0.in_queue)
    }

    pub fn out_queue(&self) -> Arc<ArrayQueue<FromFactoryMessage>> {
        Arc::clone(&self.0.out_queue)
    }
}

// BasicFactory is a simple factory for loading and managing assets.
// It uses a queue to receive load requests and another queue to send out loaded assets.
// It is designed to be used with a specific asset type, in this case, AudioW
pub struct BasicFactory<T> {
    // Storing assets in the heap allows safely sharing pointers
    // across threads for read access.
    storage: HashMap<AssetID, NonNull<T>>,
    in_queue: Option<Arc<ArrayQueue<ToFactoryMessage>>>,
    out_queue: Option<Arc<ArrayQueue<FromFactoryMessage>>>,
}

impl<T: 'static> BasicFactory<T> {
    pub fn new() -> Self {
        BasicFactory {
            storage: HashMap::new(),
            in_queue: None,
            out_queue: None,
        }
    }

    fn send(&self, message: FromFactoryMessage) {
        if let Some(out_queue) = &self.out_queue {
            out_queue.push(message).unwrap()
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        self.in_queue = Some(binding.in_queue());
        self.out_queue = Some(binding.out_queue());
    }

    pub fn process_events<F, P>(&mut self, parse: P, free: F)
    where
        P: Fn(LoadFactoryMessage) -> Result<(T, AssetMemoryUsage), String>,
        F: Fn(&T),
    {
        if let Some(in_queue) = &self.in_queue {
            while let Some(msg) = in_queue.pop() {
                match msg {
                    ToFactoryMessage::Load(message) => {
                        let aid = message.asset_id.clone();
                        let task_id = message.task_id.clone();
                        match parse(message) {
                            Ok((object, usage)) => {
                                // Move the parsed asset to the Heap and take ir pointer of it.
                                let ptr = NonNull::new(Box::into_raw(Box::new(object))).unwrap();
                                // Store the asset in the storage
                                self.storage.insert(aid.clone(), ptr);
                                self.send(FromFactoryMessage::Loaded(LoadedFactoryMessage {
                                    task_id,
                                    asset_id: aid,
                                    usage,
                                    asset_type: TypeId::of::<T>(),
                                    asset_ptr: ptr.cast(),
                                }));
                            }

                            Err(e) => {
                                self.send(FromFactoryMessage::LoadFailed(
                                    LoadFailedFactoryMessage {
                                        task_id,
                                        asset_id: aid,
                                        error: e,
                                    },
                                ));
                            }
                        }
                    }
                    ToFactoryMessage::Free(message) => {
                        let aid = message.asset_id.clone();
                        let task_id = message.task_id.clone();

                        let asset = self.storage.get(&aid).unwrap();

                        // Restore the Box from the raw pointer
                        let boxed = unsafe { Box::from_raw(asset.cast::<T>().as_ptr()) };

                        // Call the free function to clean up the asset
                        free(&*boxed);
                        // Remove the asset from the storage
                        self.storage.remove(&aid);

                        self.send(FromFactoryMessage::Freed(FreedFactoryMessage {
                            task_id,
                            asset_id: aid,
                        }));

                        // Box will be dropped here, freeing the memory
                    }
                }
            }
        }
    }
}

impl<T> Drop for BasicFactory<T> {
    fn drop(&mut self) {
        /* Warn if there's unprocessed events */
        if let Some(in_queue) = &self.in_queue {
            if !in_queue.is_empty() {
                warn!("Factory dropped with unprocessed events in the queue.");
            }
        }

        /* Warn if there's unfreed resources */
        if !self.storage.is_empty() {
            warn!(
                "Factory dropped with unfreed resources: {:?}",
                self.storage.keys()
            );
        }
    }
}
