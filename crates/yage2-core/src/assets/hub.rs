use crate::assets::factory::{AssetQueryID, FactoryBinding, InMessage, OutMessage};
use crate::assets::reader::AssetReader;
use crate::assets::registry::{AssetContainer, AssetRegistry, AssetState, QueriesRegistry};
use crate::assets::{Asset, AssetID, AssetType};
use crate::ecs::Tick;
use crossbeam_queue::ArrayQueue;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::prelude::World;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Capacity of the queue for messages sent to the asset factory.
const IN_QUEUE_CAPACITY: usize = 100;
/// Capacity of the queue for messages sent from the asset factory.
const OUT_QUEUE_CAPACITY: usize = 100;

#[derive(GlobalEvent)]
pub enum AssetHubEvent {
    QueryCompleted(AssetQueryID),
    AssetLoaded(AssetID),
    AssetFreed(AssetID),
    AllAssetsLoaded,
    AllAssetsFreed,
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
#[derive(Debug)]
pub enum QueryAssetError {
    AssetNotFound(AssetID),
    StillInUse(AssetID, usize), // AssetID and reference count
    AlreadyLoaded(AssetID),
    AlreadyFreed(AssetID),
    FactoryNotFound(AssetType),
    IPCError,
}

impl std::fmt::Display for QueryAssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryAssetError::AssetNotFound(id) => write!(f, "Asset with ID {} not found", id),
            QueryAssetError::StillInUse(id, rc) => {
                write!(f, "Asset {} is still in use (rc: {})", id, rc)
            }
            QueryAssetError::AlreadyLoaded(id) => {
                write!(f, "Asset with ID {} is already loaded", id)
            }
            QueryAssetError::AlreadyFreed(id) => write!(f, "Asset with ID {} is already freed", id),
            QueryAssetError::FactoryNotFound(asset_type) => {
                write!(f, "Factory for asset type {:?} not found", asset_type)
            }
            QueryAssetError::IPCError => write!(f, "Inter-process communication error"),
        }
    }
}

impl std::error::Error for QueryAssetError {}

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
    out_queue: Arc<ArrayQueue<OutMessage>>,
    registry: AssetRegistry,
    queries: QueriesRegistry,
}

impl AssetHub {
    pub fn new<R: AssetReader>(mut reader: R) -> Result<Self, String> {
        let mut registry = AssetRegistry::new();
        for (item_id, header) in reader.enumerate()? {
            let raw = reader.load(item_id.clone())?;
            registry.push(item_id.clone(), raw, header);
        }

        Ok(AssetHub {
            factories: HashMap::new(),
            out_queue: Arc::new(ArrayQueue::new(OUT_QUEUE_CAPACITY)),
            registry,
            queries: QueriesRegistry::new(),
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

    fn send_message(factory: &FactoryBinding, message: InMessage) -> Result<(), QueryAssetError> {
        if factory.in_queue().push(message).is_err() {
            Err(QueryAssetError::IPCError)
        } else {
            Ok(())
        }
    }

    fn select_factory(&self, id: &AssetID) -> Result<&FactoryBinding, QueryAssetError> {
        if let Some(item) = self.registry.get(id) {
            if let Some(factory) = self.factories.get(&item.header.asset_type) {
                Ok(factory)
            } else {
                Err(QueryAssetError::FactoryNotFound(
                    item.header.asset_type.clone(),
                ))
            }
        } else {
            Err(QueryAssetError::AssetNotFound(id.clone()))
        }
    }

    fn select_item(&self, id: &AssetID) -> Result<&AssetContainer, QueryAssetError> {
        self.registry
            .get(id)
            .ok_or_else(|| QueryAssetError::AssetNotFound(id.clone()))
    }

    fn query_load_inner(
        &self,
        qid: AssetQueryID,
        aid: AssetID,
    ) -> Result<AssetQueryID, QueryAssetError> {
        let factory = self.select_factory(&aid)?;
        let item = self.select_item(&aid)?;
        match &item.state {
            AssetState::Raw(raw) => {
                // If the asset is raw, we need to load it
                let message = InMessage::Load(qid, aid.clone(), raw.clone(), item.header.clone());
                Self::send_message(factory, message)?;
            }
            _ => {
                // If the asset is already loaded or freed, we can return an error
                return Err(QueryAssetError::AlreadyLoaded(aid.clone()));
            }
        }

        debug!("Query {} sent for asset {}", qid, aid);
        self.queries.add_query(qid.clone());
        Ok(qid)
    }

    pub fn query_free_inner(
        &self,
        qid: AssetQueryID,
        aid: AssetID,
    ) -> Result<AssetQueryID, QueryAssetError> {
        let factory = self.select_factory(&aid)?;
        let item = self.select_item(&aid)?;
        match &item.state {
            // If the asset is loaded, we can free it
            AssetState::Loaded(_, _) => {
                // If the asset is still in use, we cannot free it
                let rc = item.rc.load(Ordering::SeqCst);
                if rc > 0 {
                    return Err(QueryAssetError::StillInUse(aid.clone(), rc));
                }

                let message = InMessage::Free(qid, aid.clone());
                Self::send_message(factory, message)?;
            }
            _ => {
                // If the asset is not loaded, we can return an error
                return Err(QueryAssetError::AlreadyFreed(aid.clone()));
            }
        }

        debug!("Query {} sent for asset {}", qid, aid);
        self.queries.add_query(qid.clone());
        Ok(qid)
    }

    fn query_load(&self, id: AssetID) -> Result<AssetQueryID, QueryAssetError> {
        let qid = AssetQueryID::new();
        self.query_load_inner(qid, id)
    }

    pub fn query_load_all(&self) -> Result<AssetQueryID, QueryAssetError> {
        // let qid = AssetQueryID::new();
        // TODO: Batch queries is not implemented yet.
        for id in self.registry.keys() {
            self.query_load_inner(AssetQueryID::new(), id.clone())?;
        }
        Ok(AssetQueryID::new())
    }

    pub fn query_free(&self, id: AssetID) -> Result<AssetQueryID, QueryAssetError> {
        let qid = AssetQueryID::new();
        self.query_free_inner(qid, id)
    }

    pub fn query_free_all(&mut self) -> Result<AssetQueryID, QueryAssetError> {
        // let qid = AssetQueryID::new();
        // TODO: Batch queries is not implemented yet.
        for id in self.registry.keys() {
            let qid = AssetQueryID::new();
            self.query_free_inner(qid.clone(), id.clone())?;
        }
        Ok(AssetQueryID::new())
    }

    /// Retrieves an asset by its ID.
    /// If the asset is loaded, it returns an `Asset` instance.
    /// If the asset is not found or not loaded, it returns an error.
    pub fn get(&self, id: AssetID) -> Result<Asset, GetAssetError> {
        if let Some(item) = self.registry.get(&id) {
            match &item.state {
                AssetState::Loaded(tid, ptr) => {
                    let rc = Arc::clone(&item.rc);
                    let asset = Asset::new(tid.clone(), rc, ptr.clone());
                    Ok(asset)
                }
                AssetState::Freed | AssetState::Raw(_) => Err(GetAssetError::NotLoaded(id.clone())),
            }
        } else {
            Err(GetAssetError::NotFound(id))
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
            mut manager: Single<&mut AssetHub>,
            mut sender: Sender<AssetHubEvent>,
        ) {
            while let Some(message) = manager.out_queue.pop() {
                match message {
                    OutMessage::Loaded(qid, aid, tid, ptr) => {
                        debug!("Query {} loaded asset {}", qid, aid);
                        if let Some(item) = manager.registry.get_mut(&aid) {
                            item.state = AssetState::Loaded(tid, ptr);
                        } else {
                            warn!("Received Loaded message for unknown asset ID: {}", aid);
                            continue;
                        }

                        sender.send(AssetHubEvent::AssetLoaded(aid.clone()));
                        sender.send(AssetHubEvent::QueryCompleted(qid.clone()));
                        manager.queries.remove_query(&qid);

                        if manager.registry.all_loaded() {
                            info!("All assets loaded");
                            sender.send(AssetHubEvent::AllAssetsLoaded);
                        }
                    }

                    OutMessage::Freed(qid, aid) => {
                        debug!("Query {} freed asset {}", qid, aid);
                        if let Some(item) = manager.registry.get_mut(&aid) {
                            item.state = AssetState::Freed;
                        } else {
                            warn!("Received Freed message for unknown asset ID: {}", aid);
                        }

                        sender.send(AssetHubEvent::AssetFreed(aid.clone()));
                        sender.send(AssetHubEvent::QueryCompleted(qid.clone()));
                        manager.queries.remove_query(&qid);

                        if manager.registry.all_freed() {
                            info!("All assets loaded");
                            sender.send(AssetHubEvent::AllAssetsFreed);
                        }
                    }
                }
            }
        }

        world.add_handler(tick_handler);
    }
}
