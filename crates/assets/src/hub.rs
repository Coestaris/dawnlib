use crate::factory::{FactoryBinding, FromFactoryMessage, LoadFactoryMessage, ToFactoryMessage};
use crate::reader::{FromReaderMessage, ReaderBinding, ToReaderMessage};
use crate::registry::{AssetRegistry, AssetState};
use crate::requests::scheduler::{PeekResult, Scheduler, TaskDoneResult};
use crate::requests::task::{AssetTaskID, TaskCommand};
use crate::requests::{AssetRequest, AssetRequestID};
use crate::{Asset, AssetCastable, AssetHeader, AssetID, AssetMemoryUsage, AssetType, TypedAsset};
use dawn_ecs::events::TickEvent;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::World;
use log::{debug, error, info};
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;
use thiserror::Error;

/// AssetHub events are used to notify the ECS world about asset-related events.
/// These events can be used to track the status
/// of asset requests, loading, and freeing operations
#[derive(GlobalEvent)]
pub enum AssetHubEvent {
    RequestFinished(AssetRequestID, anyhow::Result<()>), // Request ID and success status
    AssetRead(AssetID),
    AssetLoaded(AssetID),
    AssetFreed(AssetID),
}

/// Error type for retrieving assets from the AssetHub.
#[derive(Error, Debug, Clone)]
pub enum GetAssetError {
    #[error("Asset with ID {0} not found")]
    NotFound(AssetID),
    #[error("Asset with ID {0} is not loaded")]
    NotLoaded(AssetID),
}

#[derive(Debug, Error, Clone)]
pub enum HubError {
    #[error("Cannot enumerate assets while some are still loaded or in use")]
    EnumerateWhileInUse,
    #[error("Cannot free asset with ID {0} while it is still in use ({1} references)")]
    AssetInUse(AssetID, usize),
    #[error("Invalid asset state for ID {0}")]
    InvalidAssetState(AssetID),
    #[error("Factory for asset type {0:?} not found")]
    FactoryNotFound(AssetType),
    #[error("Reader binding is not registered")]
    ReaderNotRegistered,
    #[error("Registry error: {0}")]
    RegistryError(#[from] crate::registry::RegistryError),
    #[error("Dependencies collect error: {0}")]
    DependenciesError(#[from] GetAssetError),
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

    pub fn try_recv(storage: Option<&Self>) -> Option<FromReaderMessage> {
        match storage {
            Some(s) => match s.receiver.try_recv() {
                Ok(message) => {
                    debug!("Received message {:?}", message);
                    Some(message)
                }
                Err(crossbeam_channel::TryRecvError::Empty) => None,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    panic!("Reader channel disconnected")
                }
            },
            None => {
                error!("Reader binding is not registered");
                None
            }
        }
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

    fn try_recv(&self) -> Option<FromFactoryMessage> {
        match self.receiver.try_recv() {
            Ok(message) => {
                debug!("Received message {:?}", message);
                Some(message)
            }
            Err(crossbeam_channel::TryRecvError::Empty) => None,
            Err(crossbeam_channel::TryRecvError::Disconnected) => None,
        }
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
    scheduler: Scheduler,
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
    pub fn new() -> Self {
        AssetHub {
            reader: None,
            factories: HashMap::new(),
            registry: AssetRegistry::new(),
            scheduler: Scheduler::new(),
        }
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
        self.scheduler.request(request)
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
    /// The main tick handler for the AssetHub.
    /// This is called on each main loop tick and processes pending tasks,
    /// reader events, and factory events.
    fn tick_handler(
        _: Receiver<TickEvent>,
        mut hub: Single<&mut AssetHub>,
        mut sender: Sender<AssetHubEvent>,
    ) {
        // Peek tasks and route to the the factories
        loop {
            let next = {
                // TODO: Temporary workaround to satisfy the borrow checker.
                //       We need to pass a reference to the registry to the scheduler,
                let registry_ptr = &hub.registry as *const AssetRegistry;
                hub.scheduler.peek(unsafe { &*registry_ptr })
            };
            match next {
                PeekResult::Peeked(task) => {
                    let result = match task.command {
                        TaskCommand::Enumerate => hub.send_enumerate(task.id.clone()),
                        TaskCommand::Read(aid) => hub.send_read(task.id.clone(), aid),
                        TaskCommand::Load(aid) => hub.send_load(task.id.clone(), aid),
                        TaskCommand::Free(aid) => hub.send_free(task.id.clone(), aid),
                    };
                    if let Err(err) = result {
                        hub.task_finished(task.id.clone(), Err(err.into()), &mut sender);
                    }
                }
                PeekResult::NoPendingTasks => break,
                PeekResult::EmptyUnwrap(tid) => {
                    // This should never happen, but just in case
                    hub.task_finished(tid, Ok(()), &mut sender);
                }
                PeekResult::UnwrapFailed(tid, err) => {
                    hub.task_finished(tid, Err(err.into()), &mut sender);
                }
            }
        }

        // Process events from the reader
        while let Some(message) = ReadStorage::try_recv(hub.reader.as_ref()) {
            hub.recv_reader(message, &mut sender);
        }

        // Process events from factories
        // Since we're sharing 'from' queue between factories,
        // we can just poll any of the binding
        let mut vec: SmallVec<[FromFactoryMessage; 8]> = smallvec![];
        for factory in hub.factories.values() {
            while let Some(message) = factory.try_recv() {
                vec.push(message);
            }
        }
        for message in vec {
            hub.recv_factory(message, &mut sender);
        }
    }

    /// Processes the task completion.
    /// This updates the task pool and notifies the ECS world about the completed request.
    fn task_finished(
        &mut self,
        tid: AssetTaskID,
        result: anyhow::Result<()>,
        sender: &mut Sender<AssetHubEvent>,
    ) {
        // Update the task pool with the completed task
        match self.scheduler.task_finished(tid, result) {
            TaskDoneResult::RequestFinished(rid, result) => {
                // Notify the ECS world about the completed request
                sender.send(AssetHubEvent::RequestFinished(
                    rid,
                    result.map_err(|e| e.into()),
                ))
            }
            _ => {}
        }
    }

    /// Receives messages from the reader and processes them.
    /// This updates the asset registry and notifies the ECS world about the asset state changes.
    fn recv_reader(&mut self, message: FromReaderMessage, mut sender: &mut Sender<AssetHubEvent>) {
        match message {
            FromReaderMessage::Enumerate(tid, Ok(headers)) => {
                // Register all headers in the registry
                self.registry.enumerate(headers);
                self.task_finished(tid, Ok(()), &mut sender);
            }
            FromReaderMessage::Enumerate(tid, Err(err)) => {
                self.task_finished(tid, Err(err), &mut sender);
            }
            FromReaderMessage::Read(tid, aid, Ok(ir)) => {
                // Save the IR asset to the registry
                self.registry
                    .update(aid.clone(), AssetState::Read(ir))
                    .unwrap();
                // Notify the ECS world about the read asset
                sender.send(AssetHubEvent::AssetRead(aid.clone()));
                self.task_finished(tid, Ok(()), &mut sender);
            }
            FromReaderMessage::Read(tid, _, Err(err)) => {
                self.task_finished(tid, Err(err), &mut sender);
            }
        };
    }

    /// Receives messages from the factories and processes them.
    /// This updates the asset registry and notifies the ECS world about the asset state changes.
    fn recv_factory(
        &mut self,
        message: FromFactoryMessage,
        mut sender: &mut Sender<AssetHubEvent>,
    ) {
        match message {
            FromFactoryMessage::Load(tid, aid, Ok(message)) => {
                self.registry
                    .update(
                        aid.clone(),
                        AssetState::Loaded(
                            Asset::new(message.asset_type, message.asset_ptr),
                            message.usage,
                        ),
                    )
                    .unwrap();

                // Notify the ECS world about the loaded asset
                sender.send(AssetHubEvent::AssetLoaded(aid.clone()));
                self.task_finished(tid, Ok(()), &mut sender);
            }
            FromFactoryMessage::Load(tid, _aid, Err(err)) => {
                self.task_finished(tid, Err(err), &mut sender);
            }
            FromFactoryMessage::Free(tid, aid, Ok(())) => {
                self.registry
                    .update(aid.clone(), AssetState::Empty)
                    .unwrap();
                // Notify the ECS world about the loaded asset
                sender.send(AssetHubEvent::AssetFreed(aid.clone()));
                self.task_finished(tid, Ok(()), &mut sender);
            }
            FromFactoryMessage::Free(tid, _aid, Err(err)) => {
                self.task_finished(tid, Err(err), &mut sender);
            }
        };
    }

    /// Sends an enumerate request to the reader.
    fn send_enumerate(&mut self, task_id: AssetTaskID) -> Result<(), HubError> {
        let reader = self.reader.as_ref().ok_or(HubError::ReaderNotRegistered)?;
        for id in self.registry.keys() {
            if let AssetState::Loaded(_, _) = self.registry.get_state(id)? {
                return Err(HubError::EnumerateWhileInUse);
            }
        }
        reader.send(ToReaderMessage::Enumerate(task_id.clone()));
        Ok(())
    }

    /// Sends a read request to the reader.
    fn send_read(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), HubError> {
        let reader = self.reader.as_ref().ok_or(HubError::ReaderNotRegistered)?;
        reader.send(ToReaderMessage::Read(task_id.clone(), id.clone()));
        Ok(())
    }

    /// Sends a load request to the appropriate factory.
    fn send_load(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), HubError> {
        let header = self.registry.get_header(&id)?;
        if let AssetState::Read(ir) = self.registry.get_state(&id)? {
            let factory = self
                .factories
                .get(&header.asset_type)
                .ok_or(HubError::FactoryNotFound(header.asset_type))?;

            // Collect dependencies
            let mut dependencies = HashMap::new();
            for dep in &header.dependencies {
                dependencies.insert(dep.clone(), self.get(dep.clone())?);
            }

            factory.send(ToFactoryMessage::Load(
                task_id.clone(),
                id.clone(),
                LoadFactoryMessage {
                    asset_header: header.clone(),
                    ir: ir.clone(),
                    dependencies,
                },
            ));
            Ok(())
        } else {
            Err(HubError::InvalidAssetState(id))
        }
    }

    /// Sends a free request to the appropriate factory.
    fn send_free(&mut self, task_id: AssetTaskID, id: AssetID) -> Result<(), HubError> {
        let header = self.registry.get_header(&id)?;
        if let AssetState::Loaded(asset, _) = self.registry.get_state(&id)? {
            if asset.ref_count() > 1 {
                return Err(HubError::AssetInUse(id, asset.ref_count()));
            }

            let factory = self
                .factories
                .get(&header.asset_type)
                .ok_or(HubError::FactoryNotFound(header.asset_type))?;

            factory.send(ToFactoryMessage::Free(task_id.clone(), id.clone()));
            Ok(())
        } else {
            Err(HubError::InvalidAssetState(id))
        }
    }
}
