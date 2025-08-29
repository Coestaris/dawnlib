use crate::binding::Binding;
use crate::ir::IRAsset;
use crate::{Asset, AssetHeader, AssetID, AssetMemoryUsage, AssetType};
use crossbeam_channel::{Receiver, Sender};
use log::{error, warn};
use std::any::TypeId;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::time::Duration;
use crate::requests::task::AssetTaskID;

#[derive(Debug)]
pub struct LoadFactoryMessage {
    pub asset_header: AssetHeader,
    pub ir: IRAsset,
    pub dependencies: HashMap<AssetID, Asset>,
}

#[derive(Debug)]
pub enum ToFactoryMessage {
    Load(AssetTaskID, AssetID, LoadFactoryMessage),
    Free(AssetTaskID, AssetID),
}

#[derive(Debug)]
pub struct LoadedFactoryMessage {
    pub usage: AssetMemoryUsage,
    pub asset_type: TypeId,
    pub asset_ptr: NonNull<()>,
}

#[derive(Debug)]
pub enum FromFactoryMessage {
    Load(AssetTaskID, AssetID, anyhow::Result<LoadedFactoryMessage>),
    Free(AssetTaskID, AssetID, anyhow::Result<()>),
}

// Make rust happy with sending NonNull
unsafe impl Send for FromFactoryMessage {}
unsafe impl Send for ToFactoryMessage {}
unsafe impl Sync for FromFactoryMessage {}
unsafe impl Sync for ToFactoryMessage {}

pub struct FactoryBinding {
    asset_type: AssetType,
    inner: Binding<ToFactoryMessage, FromFactoryMessage>,
}

impl FactoryBinding {
    pub(crate) fn new(
        asset_type: AssetType,
    ) -> (Self, Sender<ToFactoryMessage>, Receiver<FromFactoryMessage>) {
        let (inner, to_sender, from_receiver) = Binding::new();
        (
            FactoryBinding { asset_type, inner },
            to_sender,
            from_receiver,
        )
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn send(&self, message: FromFactoryMessage) {
        self.inner.send(message);
    }

    pub fn recv(&self, timeout: Duration) -> Option<ToFactoryMessage> {
        self.inner.recv(timeout)
    }
}

// BasicFactory is a simple factory for loading and managing assets.
// It uses a queue to receive load requests and another queue to send out loaded assets.
// It is designed to be used with a specific asset type
pub struct BasicFactory<T> {
    // Storing assets in the heap allows safely sharing pointers
    // across threads for read access.
    storage: HashMap<AssetID, NonNull<T>>,
    binding: Option<FactoryBinding>,
}

impl<T: 'static> BasicFactory<T> {
    pub fn new() -> Self {
        BasicFactory {
            storage: HashMap::new(),
            binding: None,
        }
    }

    fn send(&self, message: FromFactoryMessage) {
        if let Some(binding) = &self.binding {
            binding.send(message);
        }
    }

    fn recv(&self, timeout: Duration) -> Option<ToFactoryMessage> {
        if let Some(binding) = &self.binding {
            binding.recv(timeout)
        } else {
            None
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        self.binding = Some(binding);
    }

    pub fn process_events<F, P>(&mut self, parse: P, free: F, timeout: Duration)
    where
        P: Fn(LoadFactoryMessage) -> anyhow::Result<(T, AssetMemoryUsage)>,
        F: Fn(&T),
    {
        if self.binding.is_none() {
            error!("Factory not bound to any queues.");
            return;
        }

        while let Some(msg) = self.recv(timeout) {
            match msg {
                ToFactoryMessage::Load(tid, aid, payload) => {
                    let tid = tid.clone();
                    let aid = aid.clone();
                    match parse(payload) {
                        Ok((object, usage)) => {
                            // Move the parsed asset to the Heap and take ir pointer of it.
                            let ptr = NonNull::new(Box::into_raw(Box::new(object))).unwrap();
                            // Store the asset in the storage
                            self.storage.insert(aid.clone(), ptr);
                            self.send(FromFactoryMessage::Load(
                                tid,
                                aid,
                                Ok(LoadedFactoryMessage {
                                    usage,
                                    asset_type: TypeId::of::<T>(),
                                    asset_ptr: ptr.cast(),
                                }),
                            ));
                        }

                        Err(e) => {
                            self.send(FromFactoryMessage::Load(tid, aid, Err(e)));
                        }
                    }
                }
                ToFactoryMessage::Free(tid, aid) => {
                    let aid = aid.clone();
                    let tid = tid.clone();

                    let asset = self.storage.get(&aid).unwrap();

                    // Restore the Box from the raw pointer
                    let boxed = unsafe { Box::from_raw(asset.cast::<T>().as_ptr()) };

                    // Call the free function to clean up the asset
                    free(&*boxed);
                    // Remove the asset from the storage
                    self.storage.remove(&aid);

                    self.send(FromFactoryMessage::Free(tid, aid, Ok(())));

                    // Box will be dropped here, freeing the memory
                }
            }
        }
    }
}

impl<T> Drop for BasicFactory<T> {
    fn drop(&mut self) {
        /* Warn if there's unfreed resources */
        if !self.storage.is_empty() {
            warn!(
                "Factory dropped with unfreed resources: {:?}",
                self.storage.keys()
            );
        }
    }
}
