use crate::factory::{FactoryBinding, FromFactoryMessage, LoadFactoryMessage, ToFactoryMessage};
use crate::reader::{FromReaderMessage, ReaderBinding, ToReaderMessage};
use crate::registry::{AssetRegistry, AssetState};
use crate::requests::pool::{PeekResult, TaskDoneResult, TaskPool};
use crate::requests::task::{AssetTaskID, TaskCommand};
use crate::requests::{AssetRequest, AssetRequestID};
use crate::{Asset, AssetCastable, AssetHeader, AssetID, AssetMemoryUsage, AssetType, TypedAsset};
use dawn_ecs::Tick;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::World;
use log::{debug, error, info};
use std::collections::HashMap;
use thiserror::Error;

/// AssetHub events are used to notify the ECS world about asset-related events.
/// These events can be used to track the status
/// of asset requests, loading, and freeing operations
#[derive(GlobalEvent)]
pub enum AssetHubEvent {
    RequestCompleted(AssetRequestID, Result<(), String>), // Request ID and success status
    AssetFailed(AssetID, Option<String>),                 // Asset ID and optional error message
    AssetLoaded(AssetID),
    AssetFreed(AssetID),
}

/// Error type for retrieving assets from the AssetHub.
#[derive(Debug)]
pub enum GetAssetError {
    NotFound(AssetID),
    NotLoaded(AssetID),
}

impl std::fmt::Display for GetAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetAssetError::NotFound(id) => write!(f, "Asset with ID {} not found", id),
            GetAssetError::NotLoaded(id) => write!(f, "Asset with ID {} is not loaded", id),
        }
    }
}

impl std::error::Error for GetAssetError {}

#[derive(Debug, Error, Clone)]
pub enum RequestAssetError {
    #[error("Asset with ID {0} is not registered in the AssetHub")]
    AssetNotFound(AssetID),
    #[error("Asset with ID {0} is still in use, reference count: {1}")]
    StillInUse(AssetID, usize), // AssetID and reference count
    #[error("Asset with ID {0} is not in IR state, cannot load it")]
    AlreadyLoaded(AssetID),
    #[error("Asset with ID {0} is not in Loaded state, cannot free it")]
    AlreadyFreed(AssetID),
    #[error("Asset type {0} is not supported by the AssetHub")]
    FactoryNotFound(AssetType),
    #[error("IPC error occurred")]
    IPCError,
}

struct ReadStorage {
    sender: crossbeam_channel::Sender<ToReaderMessage>,
    receiver: crossbeam_channel::Receiver<FromReaderMessage>,
}

impl ReadStorage {
    fn new() -> (Self, ReaderBinding) {
        let (binding, sender, receiver) = ReaderBinding::new();
        (Self { sender, receiver }, binding)
    }

    fn send(&self, message: ToReaderMessage) {
        debug!("Sending message: {:?}", message);
        self.sender.send(message).unwrap();
    }

    fn recv(&self, max: usize) -> Vec<FromReaderMessage> {
        let mut messages = Vec::new();
        for _ in 0..max {
            if let Ok(message) = self.receiver.try_recv() {
                debug!("Received message {:?}", message);
                messages.push(message);
            } else {
                break;
            }
        }
        messages
    }
}

struct FactoryStorage {
    sender: crossbeam_channel::Sender<ToFactoryMessage>,
    receiver: crossbeam_channel::Receiver<FromFactoryMessage>,
}

impl FactoryStorage {
    fn new(asset_type: AssetType) -> (Self, FactoryBinding) {
        let (binding, sender, receiver) = FactoryBinding::new(asset_type);
        (Self { sender, receiver }, binding)
    }

    fn send(&self, message: ToFactoryMessage) {
        debug!("Sending message: {:?}", message);
        self.sender.send(message).unwrap();
    }

    fn recv(&self, max: usize) -> Vec<FromFactoryMessage> {
        let mut messages = Vec::new();
        for _ in 0..max {
            if let Ok(message) = self.receiver.try_recv() {
                debug!("Received message {:?}", message);
                messages.push(message);
            } else {
                break;
            }
        }

        messages
    }
}

/// The AssetHub is the main entry point for managing assets in the system.
///
/// It relies on factories to load and free assets asynchronously using queues.
/// The assets memory management is also handled by factories - they provide raw
/// pointers to the loaded assets, which can be used to access the asset data.
/// The AssetHub keeps track of the assets usage using reference counting,
/// allowing safe multithreading read-only access to the assets.
///
/// To control the assets, you create requests, by 'request' methods,
/// The status of the requests can be tracked using the `AssetHubEvent`
/// sent to the ECS world.
#[derive(Component)]
pub struct AssetHub {
    reader: Option<ReadStorage>,
    factories: HashMap<AssetType, FactoryStorage>,
    registry: AssetRegistry,
    task_pool: TaskPool,
}

#[derive(Debug, Clone)]
pub enum AssetInfoState {
    Empty,
    IR(usize), // Ram usage
    Loaded { usage: AssetMemoryUsage, rc: usize },
}

#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub id: AssetID,
    pub header: AssetHeader,
    pub state: AssetInfoState,
}

impl AssetHub {
    pub fn new() -> Result<Self, String> {
        Ok(AssetHub {
            reader: None,
            factories: HashMap::new(),
            registry: AssetRegistry::new(),
            task_pool: TaskPool::new(),
        })
    }

    /// Creates a new factory binding for the specified asset type.
    /// Binding is the generic interface for bidirectional communication
    /// between the AssetHub and the asset factory.
    pub fn get_factory_biding(&mut self, asset_type: AssetType) -> FactoryBinding {
        if self.factories.contains_key(&asset_type) {
            panic!("Factory for asset type {:?} already registered", asset_type);
        }

        let (storage, binding) = FactoryStorage::new(asset_type);
        self.factories.insert(asset_type, storage);

        info!("Creating factory binding for asset type {:?}", asset_type);
        binding
    }

    /// Creates a read binding used to communicate between the AssetHub and the asset reader.
    /// Read binding is the generic interface for bidirectional communication
    /// between the AssetHub and the asset reader.
    pub fn get_read_binding(&mut self) -> ReaderBinding {
        if self.reader.is_some() {
            panic!("Reader binding already registered");
        }

        let (storage, binding) = ReadStorage::new();
        self.reader = Some(storage);

        info!("Creating reader binding");
        binding
    }

    /// Lazily requests some action.
    /// All requests are guaranteed to be executed in the order they were requested.
    /// Returns a unique ID for the request that can be used to track its status.
    /// You can safely reference the assets that is not enumerated yet,
    /// since the actual validation and execution of the request is deferred
    /// until the main loop tick, when the AssetHub is processed.
    /// If the request cannot be fulfilled (e.g. asset not found, circular dependency),
    /// the request will fail and an `AssetHubEvent::RequestCompleted` event will be
    /// sent to the ECS world with the error message.
    pub fn request(&mut self, request: AssetRequest) -> AssetRequestID {
        debug!("Requesting: {:?}", request);
        self.task_pool.request(request)
    }

    /// Retrieves an asset by its ID.
    /// If the asset is loaded, it returns an `Asset` instance.
    /// If the asset is not found or not loaded, it returns an error.
    pub fn get(&self, id: AssetID) -> Result<Asset, GetAssetError> {
        match self
            .registry
            .get_state(&id)
            .map_err(|_| GetAssetError::NotFound(id.clone()))?
        {
            AssetState::Loaded(asset, _) => Ok(asset.clone()),
            AssetState::Empty | AssetState::Read(_) => Err(GetAssetError::NotLoaded(id.clone())),
        }
    }

    /// Retrieves a typed asset by its ID.
    /// Typed assets are wrappers around the `Asset` type that provide
    /// type-safe access to the asset data.
    pub fn get_typed<T: AssetCastable>(&self, id: AssetID) -> Result<TypedAsset<T>, GetAssetError> {
        Ok(TypedAsset::new(self.get(id)?))
    }

    /// Moves the Asset Hub into the ECS world.
    /// This will allow automatically processing async events on each main loop tick.
    /// This also will provide additional ECS events as `AssetHubEvent` that can be
    /// used to track requests status and other asset-related events.
    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
        world.add_handler(Self::tick_handler.low());
    }

    /// Collect debug information about all enumerated assets.
    /// This includes asset ID, header, and current state (Empty, IR, Loaded).
    pub fn asset_infos(&self) -> Vec<AssetInfo> {
        let mut infos = Vec::new();
        for id in self.registry.keys() {
            infos.push(AssetInfo {
                id: id.clone(),
                header: self.registry.get_header(id).unwrap().clone(),
                state: match self.registry.get_state(&id).unwrap() {
                    AssetState::Empty => AssetInfoState::Empty,
                    AssetState::Read(ir) => AssetInfoState::IR(ir.memory_usage()),
                    AssetState::Loaded(asset, usage) => AssetInfoState::Loaded {
                        usage: usage.clone(),
                        rc: asset.ref_count(),
                    },
                },
            })
        }

        infos
    }
}

impl AssetHub {
    fn tick_handler(
        _: Receiver<Tick>,
        mut hub: Single<&mut AssetHub>,
        mut sender: Sender<AssetHubEvent>,
    ) {
        fn task_done(tid: AssetTaskID, hub: &mut AssetHub, sender: &mut Sender<AssetHubEvent>) {
            // Update the task pool with the completed task
            match hub.task_pool.task_done(tid) {
                TaskDoneResult::RequestCompleted(qid) => {
                    // Notify the ECS world about the completed request
                    sender.send(AssetHubEvent::RequestCompleted(qid, Ok(())));
                }
                _ => {}
            }
        }

        fn task_failed(
            tid: AssetTaskID,
            error: String,
            hub: &mut AssetHub,
            sender: &mut Sender<AssetHubEvent>,
        ) {
            // Update the task pool with the failed task
            hub.task_pool.task_failed(tid);
            // Notify the ECS world about the completed request
            sender.send(AssetHubEvent::RequestCompleted(
                tid.as_request(),
                Err(error.clone()),
            ));
        }

        // Peek tasks and route the to the factories
        loop {
            match hub.task_pool.peek_task() {
                PeekResult::Peeked(task) => {
                    let result = match task.command {
                        TaskCommand::Enumerate => hub.send_enumerate(task.id.clone()),
                        TaskCommand::Read(aid) => hub.send_read(task.id.clone(), aid),
                        TaskCommand::Load(aid) => hub.send_load(task.id.clone(), aid),
                        TaskCommand::Free(aid) => hub.send_free(task.id.clone(), aid),
                    };
                    if let Err(e) = result {
                        task_failed(task.id.clone(), e, &mut hub, &mut sender);
                    }
                }
                PeekResult::NoPendingTasks => break,
                PeekResult::UnwrapFailed(tid, message) => {
                    task_failed(
                        tid,
                        format!("Failed to unwrap task with ID {:?}: {}", tid, message),
                        &mut hub,
                        &mut sender,
                    );
                }
            }
        }

        // Process events from reader
        for message in hub.receive_from_readers(8) {
            match message {
                FromReaderMessage::Enumerate(tid, Ok(headers)) => {
                    // Register all headers in the registry
                    hub.registry.enumerate(headers);
                    task_done(tid, &mut hub, &mut sender);
                }
                FromReaderMessage::Enumerate(tid, Err(message)) => {
                    task_failed(tid, message, &mut hub, &mut sender);
                }
                FromReaderMessage::Read(tid, aid, Ok(ir)) => {
                    // Save the IR asset to the registry
                    hub.registry
                        .update(aid.clone(), AssetState::Read(ir))
                        .unwrap();
                    task_done(tid, &mut hub, &mut sender);
                }
                FromReaderMessage::Read(tid, _, Err(message)) => {
                    task_failed(tid, message, &mut hub, &mut sender);
                }
            };
        }

        // Process events from factories
        // Since we're sharing 'from' queue between factories,
        // we can just poll any of the binding
        for message in hub.receive_from_factory(8) {
            match message {
                FromFactoryMessage::Load(tid, aid, Ok(message)) => {
                    // Save the asset to the registry
                    hub.registry
                        .update(
                            aid.clone(),
                            AssetState::Loaded(
                                Asset::new(aid.clone(), message.asset_type, message.asset_ptr),
                                message.usage,
                            ),
                        )
                        .unwrap();

                    // Notify the ECS world about the loaded asset
                    sender.send(AssetHubEvent::AssetLoaded(aid.clone()));
                    task_done(tid, &mut hub, &mut sender);
                }
                FromFactoryMessage::Load(tid, aid, Err(message)) => {
                    sender.send(AssetHubEvent::AssetFailed(
                        aid.clone(),
                        Some(message.clone()),
                    ));
                    task_failed(tid, message, &mut hub, &mut sender);
                }
                FromFactoryMessage::Free(tid, aid, Ok(())) => {
                    // Save the asset to the registry
                    hub.registry.update(aid.clone(), AssetState::Empty).unwrap();
                    // Notify the ECS world about the loaded asset
                    sender.send(AssetHubEvent::AssetLoaded(aid.clone()));
                    task_done(tid, &mut hub, &mut sender);
                }
                FromFactoryMessage::Free(tid, aid, Err(message)) => {
                    task_failed(tid, message, &mut hub, &mut sender);
                }
            };
        }
    }

    fn send_enumerate(&mut self, task_id: AssetTaskID) -> Result<(), String> {
        if let Some(reader) = self.reader.as_ref() {
            for id in self.registry.keys() {
                if let AssetState::Loaded(_, _) = self.registry.get_state(id).unwrap() {
                    return Err(format!(
                        "Asset ID {} is not freed or in use. Cannot enumerate",
                        id
                    ));
                }
            }
            reader.send(ToReaderMessage::Enumerate(task_id.clone()));
            Ok(())
        } else {
            Err("No reader found".to_string())
        }
    }

    fn send_read(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), String> {
        if let Some(reader) = self.reader.as_ref() {
            if let Err(_) = self.registry.get_header(&id) {
                return Err(format!("Asset with ID {} not found", id));
            }
            reader.send(ToReaderMessage::Read(task_id.clone(), id.clone()));
            Ok(())
        } else {
            Err("No reader found".to_string())
        }
    }

    fn send_load(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), String> {
        let header = self.registry.get_header(&id)?;
        match self.registry.get_state(&id)? {
            AssetState::Read(ir) => {
                let factory = self.factories.get(&header.asset_type);
                if let Some(factory) = factory {
                    let mut dependencies = HashMap::new();
                    for dep in &header.dependencies {
                        dependencies.insert(dep.clone(), self.get(dep.clone()).unwrap());
                    }

                    let message = ToFactoryMessage::Load(
                        task_id.clone(),
                        id.clone(),
                        LoadFactoryMessage {
                            asset_header: header.clone(),
                            ir: ir.clone(),
                            dependencies,
                        },
                    );
                    factory.send(message);
                } else {
                    panic!(
                        "No factory found for asset type {:?} for asset ID {}",
                        header.asset_type, id
                    );
                }

                Ok(())
            }
            other => Err(format!(
                "Asset with ID {} is not in READ state, cannot load it: {:?}",
                id, other
            )),
        }
    }

    fn send_free(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), String> {
        let header = self.registry.get_header(&id)?;
        match self.registry.get_state(&id)? {
            AssetState::Loaded(_, _) => {
                let factory = self.factories.get(&header.asset_type);
                if let Some(factory) = factory {
                    let message = ToFactoryMessage::Free(task_id.clone(), id.clone());
                    factory.send(message);
                } else {
                    panic!(
                        "No factory found for asset type {:?} for asset ID {}",
                        header.asset_type, id
                    );
                }

                Ok(())
            }
            other => Err(format!(
                "Asset with ID {} is not in LOADED state, cannot free it: {:?}",
                id, other
            )),
        }
    }

    fn receive_from_readers(&mut self, max: usize) -> Vec<FromReaderMessage> {
        let mut messages = Vec::new();
        if let Some(reader) = &self.reader {
            messages.extend(reader.recv(max));
        }
        messages
    }

    fn receive_from_factory(&mut self, max: usize) -> Vec<FromFactoryMessage> {
        let mut messages = Vec::new();
        for factory in self.factories.values() {
            messages.extend(factory.recv(max));
        }
        messages
    }
}
