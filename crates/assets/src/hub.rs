use crate::factory::{
    FactoryBinding, FreeFactoryMessage, FromFactoryMessage, LoadFactoryMessage, ToFactoryMessage,
};
use crate::query::{AssetQueryID, AssetTaskID, QueryError, TaskCommand, TaskDoneResult, TaskPool};
use crate::reader::AssetReader;
use crate::registry::{AssetRegistry, AssetState};
use crate::{Asset, AssetCastable, AssetID, AssetMemoryUsage, AssetType, TypedAsset};
use crossbeam_queue::ArrayQueue;
use dawn_ecs::Tick;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::prelude::World;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Capacity of the queue for messages sent to the asset factory.
const IN_QUEUE_CAPACITY: usize = 100;
/// Capacity of the queue for messages sent from the asset factory.
const OUT_QUEUE_CAPACITY: usize = 100;

/// AssetHub events are used to notify the ECS world about asset-related events.
/// These events can be used to track the status
/// of asset queries, loading, and freeing operations
#[derive(GlobalEvent)]
pub enum AssetHubEvent {
    QueryCompleted(AssetQueryID, bool), // Query ID and success status
    AssetFailed(AssetID, Option<String>), // Asset ID and optional error message
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

/// Error type for querying assets in the AssetHub.
#[derive(Debug, Error, Clone)]
pub enum QueryAssetError {
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
    #[error("Query error occurred: {0}")]
    QueryError(#[from] QueryError),
    #[error("IPC error occurred")]
    IPCError,
}

/// The AssetHub is the main entry point for managing assets in the system.
///
/// It relies on factories to load and free assets asynchronously using queues.
/// The assets memory management is also handled by factories - they provide raw
/// pointers to the loaded assets, which can be used to access the asset data.
/// The AssetHub keeps track of the assets usage using reference counting,
/// allowing safe multithreading read-only access to the assets.
///
/// To control the assets, you create a queries, e.g. `query_load` or `query_free`.
/// The status of the queries can be tracked using the `AssetHubEvent`
/// sent to the ECS world.
#[derive(Component)]
pub struct AssetHub {
    factories: HashMap<AssetType, FactoryBinding>,
    out_queue: Arc<ArrayQueue<FromFactoryMessage>>,
    registry: AssetRegistry,
    task_pool: TaskPool,
}

pub enum AssetInfoState {
    Empty,
    IR(usize), // Ram usage
    Loaded { usage: AssetMemoryUsage, rc: usize },
}

pub struct AssetInfo {
    id: AssetID,
    state: AssetInfoState,
}

impl AssetHub {
    pub fn new<R: AssetReader>(mut reader: R) -> Result<Self, String> {
        let mut registry = AssetRegistry::new();
        for (item_id, (header, ir)) in reader.read()? {
            registry.register(item_id.clone(), header);
            registry.update(item_id.clone(), AssetState::IR(ir))?;
        }

        Ok(AssetHub {
            factories: HashMap::new(),
            out_queue: Arc::new(ArrayQueue::new(OUT_QUEUE_CAPACITY)),
            registry,
            task_pool: TaskPool::new(),
        })
    }

    /// Creates a new factory binding for the specified asset type.
    /// Binding is the generic interface for bidirectional communication
    /// between the AssetHub and the asset factory.
    pub fn create_factory_biding(&mut self, asset_type: AssetType) -> FactoryBinding {
        if self.factories.contains_key(&asset_type) {
            panic!("Factory for asset type {:?} already registered", asset_type);
        }

        // Setup async queues for the factory
        let in_queue = Arc::new(ArrayQueue::new(IN_QUEUE_CAPACITY));
        let out_queue = &self.out_queue;
        let binding = FactoryBinding::new(asset_type, Arc::clone(&in_queue), Arc::clone(out_queue));

        info!("Creating factory binding for asset type {:?}", asset_type);
        self.factories.insert(asset_type, binding.clone());
        binding
    }

    pub fn query_load(&mut self, id: AssetID) -> Result<AssetQueryID, QueryAssetError> {
        let qid = self.task_pool.query_load(id.clone(), &self.registry)?;
        Ok(qid)
    }

    pub fn query_load_all(&mut self) -> Result<AssetQueryID, QueryAssetError> {
        let qid = self.task_pool.query_load_all(&self.registry)?;
        Ok(qid)
    }

    pub fn query_free(&self, id: AssetID) -> Result<AssetQueryID, QueryAssetError> {
        todo!()
    }

    pub fn query_free_all(&mut self) -> Result<AssetQueryID, QueryAssetError> {
        todo!()
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
            AssetState::Empty | AssetState::IR(_) => Err(GetAssetError::NotLoaded(id.clone())),
        }
    }

    /// Retrieves a typed asset by its ID.
    /// Typed assets are wrappers around the `Asset` type that provide
    /// type-safe access to the asset data.
    pub fn get_typed<T: AssetCastable>(&self, id: AssetID) -> Result<TypedAsset<T>, GetAssetError> {
        Ok(TypedAsset::new(self.get(id)?))
    }

    fn send_load(&mut self, task_id: AssetTaskID, id: AssetID) {
        let header = self.registry.get_header(&id).unwrap();
        match self.registry.get_state(&id).unwrap() {
            AssetState::IR(ir) => {
                let factory = self.factories.get(&header.asset_type);
                if let Some(factory) = factory {
                    let mut dependencies = HashMap::new();
                    for dep in &header.dependencies {
                        dependencies.insert(dep.clone(), self.get(dep.clone()).unwrap());
                    }
                    let message = LoadFactoryMessage {
                        task_id: task_id.clone(),
                        asset_id: id.clone(),
                        asset_header: header.clone(),
                        ir: ir.clone(),
                        dependencies,
                    };
                    debug!("Sending load message {:?}", message);
                    factory
                        .in_queue()
                        .push(ToFactoryMessage::Load(message))
                        .unwrap();
                } else {
                    panic!(
                        "No factory found for asset type {:?} for asset ID {}",
                        header.asset_type, id
                    );
                }
            }
            other => {
                warn!(
                    "Asset with ID {} is not in IR state, cannot load it: {:?}",
                    id, other
                );
                return;
            }
        }
    }

    fn send_free(&mut self, task_id: AssetTaskID, id: AssetID) {
        let header = self.registry.get_header(&id).unwrap();
        match self.registry.get_state(&id).unwrap() {
            AssetState::Loaded(_, _) => {
                let factory = self.factories.get(&header.asset_type);
                if let Some(factory) = factory {
                    let message = FreeFactoryMessage {
                        task_id: task_id.clone(),
                        asset_id: id.clone(),
                    };
                    debug!("Sending free message {:?}", message);
                    factory
                        .in_queue()
                        .push(ToFactoryMessage::Free(message))
                        .unwrap();
                } else {
                    panic!(
                        "No factory found for asset type {:?} for asset ID {}",
                        header.asset_type, id
                    );
                }
            }
            other => {
                warn!(
                    "Asset with ID {} is not in Loaded state, cannot free it: {:?}",
                    id, other
                );
                return;
            }
        }
    }

    /// Moves the Asset Hub into the ECS world.
    /// This will allow automatically processing async events on each main loop tick.
    /// This also will provide additional ECS events as `AssetHubEvent` that can be
    /// used to track queries status and other asset-related events.
    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);

        fn tick_handler(
            _: Receiver<Tick>,
            mut hub: Single<&mut AssetHub>,
            mut sender: Sender<AssetHubEvent>,
        ) {
            // Peek tasks and route the to the factories
            while let Some(task) = hub.task_pool.peek_task() {
                match task.command {
                    TaskCommand::IR(_) => {
                        unimplemented!()
                    }
                    TaskCommand::Load(aid) => hub.send_load(task.id.clone(), aid),
                    TaskCommand::Free(aid) => hub.send_free(task.id.clone(), aid),
                }
            }

            // Process events from factories
            while let Some(message) = hub.out_queue.pop() {
                debug!("Received message {:?}", message);
                match message {
                    FromFactoryMessage::Loaded(message) => {
                        // Save the asset to the registry
                        hub.registry
                            .update(
                                message.asset_id.clone(),
                                AssetState::Loaded(
                                    Asset::new(
                                        message.asset_id.clone(),
                                        message.asset_type,
                                        message.asset_ptr,
                                    ),
                                    message.usage,
                                ),
                            )
                            .unwrap();
                        // Update the task pool with the completed task
                        match hub.task_pool.task_done(message.task_id) {
                            TaskDoneResult::QueryComplete(qid) => {
                                // Notify the ECS world about the completed query
                                sender.send(AssetHubEvent::QueryCompleted(qid, true));
                            }
                            _ => {}
                        }
                        // Notify the ECS world about the loaded asset
                        sender.send(AssetHubEvent::AssetLoaded(message.asset_id.clone()));
                    }
                    FromFactoryMessage::Freed(message) => {
                        // Remove the asset from the registry
                        hub.registry
                            .update(message.asset_id.clone(), AssetState::Empty)
                            .unwrap();
                        // Update the task pool with the completed task
                        match hub.task_pool.task_done(message.task_id) {
                            TaskDoneResult::QueryComplete(qid) => {
                                // Notify the ECS world about the completed query
                                sender.send(AssetHubEvent::QueryCompleted(qid, true));
                            }
                            _ => {}
                        }
                        // Notify the ECS world about the freed asset
                        sender.send(AssetHubEvent::AssetFreed(message.asset_id.clone()));
                    }
                    FromFactoryMessage::LoadFailed(message) => {
                        // Update the task pool with the failed task
                        hub.task_pool.task_failed(message.task_id);
                        // Notify the ECS world about the failed asset
                        sender.send(AssetHubEvent::QueryCompleted(
                            message.task_id.as_query_id(),
                            false,
                        ));
                        sender.send(AssetHubEvent::AssetFailed(
                            message.asset_id.clone(),
                            Some(message.error),
                        ));
                    }
                }
            }
        }

        world.add_handler(tick_handler.low());
    }

    pub fn asset_infos(&self) -> Vec<AssetInfo> {
        let mut infos = Vec::new();
        for id in self.registry.keys() {
            infos.push(AssetInfo {
                id: id.clone(),
                state: match self.registry.get_state(&id).unwrap() {
                    AssetState::Empty => AssetInfoState::Empty,
                    AssetState::IR(ir) => AssetInfoState::IR(ir.memory_usage()),
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
