use crate::assets::raw::AssetRaw;
use crate::assets::{AssetHeader, AssetID, AssetType};
use crossbeam_queue::ArrayQueue;
use log::{error, warn};
use std::any::TypeId;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetQueryID(usize);

impl std::fmt::Display for AssetQueryID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QueryID({})", self.0)
    }
}

impl AssetQueryID {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AssetQueryID(id)
    }
}

pub enum InMessage {
    Load(AssetQueryID, AssetID, AssetHeader, AssetRaw),
    Free(AssetQueryID, AssetID),
}

#[derive(Debug)]
pub enum OutMessage {
    Loaded(AssetQueryID, AssetID, TypeId, NonNull<()>),
    Failed(AssetQueryID, AssetID, String),
    Freed(AssetQueryID, AssetID),
}

// Make rust happy with sending NonNull
unsafe impl Send for OutMessage {}
unsafe impl Sync for OutMessage {}
unsafe impl Send for InMessage {}
unsafe impl Sync for InMessage {}

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

// BasicFactory is a simple factory for loading and managing assets.
// It uses a queue to receive load requests and another queue to send out loaded assets.
// It is designed to be used with a specific asset type, in this case, AudioW
pub struct BasicFactory<T> {
    // Storing assets in the heap allows safely sharing pointers
    // across threads for read access.
    storage: HashMap<AssetID, NonNull<T>>,
    in_queue: Option<Arc<ArrayQueue<InMessage>>>,
    out_queue: Option<Arc<ArrayQueue<OutMessage>>>,
}

impl<T: 'static> BasicFactory<T> {
    pub fn new() -> Self {
        BasicFactory {
            storage: HashMap::new(),
            in_queue: None,
            out_queue: None,
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        self.in_queue = Some(binding.in_queue());
        self.out_queue = Some(binding.out_queue());
    }

    pub fn process_events<F, P>(&mut self, parse: P, free: F)
    where
        P: Fn(&AssetHeader, &AssetRaw) -> Result<T, String>,
        F: Fn(&T),
    {
        if let Some(in_queue) = &self.in_queue {
            while let Some(msg) = in_queue.pop() {
                match msg {
                    InMessage::Load(qid, id, header, raw) => match parse(&header, &raw) {
                        Ok(parsed) => {
                            // Move the parsed asset to the Heap and take raw pointer of it.
                            let ptr = NonNull::new(Box::into_raw(Box::new(parsed))).unwrap();
                            // Store the asset in the storage
                            self.storage.insert(id.clone(), ptr);

                            if let Some(out_queue) = &self.out_queue {
                                out_queue
                                    .push(OutMessage::Loaded(
                                        qid,
                                        id,
                                        TypeId::of::<T>(),
                                        ptr.cast(),
                                    ))
                                    .unwrap();
                            }
                        }

                        Err(e) => {
                            if let Some(out_queue) = &self.out_queue {
                                out_queue.push(OutMessage::Failed(qid, id, e)).unwrap();
                            }
                        }
                    },
                    InMessage::Free(qid, id) => {
                        if let Some(asset) = self.storage.get(&id) {
                            // Restore the Box from the raw pointer
                            let boxed = unsafe { Box::from_raw(asset.cast::<T>().as_ptr()) };

                            // Call the free function to clean up the asset
                            free(&*boxed);
                            // Remove the asset from the storage
                            self.storage.remove(&id);

                            if let Some(out_queue) = &self.out_queue {
                                out_queue.push(OutMessage::Freed(qid, id)).unwrap();
                            }

                            // Box will be dropped here, freeing the memory
                        } else {
                            error!("Failed to free asset: {}. Asset not found.", id);
                        }
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
