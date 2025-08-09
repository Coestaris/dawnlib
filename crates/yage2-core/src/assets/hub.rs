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
use std::sync::Arc;

#[derive(GlobalEvent)]
pub enum AssetHubEvent {
    QueryCompleted(AssetQueryID),
    AssetLoaded(AssetID),
    AssetFreed(AssetID),
    AllAssetsLoaded,
    AllAssetsFreed,
}

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
            out_queue: Arc::new(ArrayQueue::new(100)), // Example size, adjust as needed
            registry,
            queries: QueriesRegistry::new(),
        })
    }

    pub fn create_factory_biding(&mut self, asset_type: AssetType) -> FactoryBinding {
        if self.factories.contains_key(&asset_type) {
            panic!("Factory for asset type {:?} already registered", asset_type);
        }

        // Setup async queues for the factory
        let in_queue = Arc::new(ArrayQueue::new(100)); // Example size, adjust as needed
        let out_queue = &self.out_queue;
        let binding = FactoryBinding::new(asset_type, Arc::clone(&in_queue), Arc::clone(out_queue));

        info!("Creating factory binding for asset type {:?}", asset_type);
        self.factories.insert(asset_type, binding.clone());
        binding
    }

    fn send_message(factory: &FactoryBinding, message: InMessage) -> Result<(), String> {
        if factory.in_queue().push(message).is_err() {
            Err("Failed to send message to factory".to_string())
        } else {
            Ok(())
        }
    }

    fn select_factory(&self, id: &AssetID) -> Result<&FactoryBinding, String> {
        if let Some(item) = self.registry.get(id) {
            if let Some(factory) = self.factories.get(&item.header.asset_type) {
                Ok(factory)
            } else {
                Err(format!(
                    "No factory registered for asset type {:?}",
                    item.header.asset_type
                ))
            }
        } else {
            Err(format!("Asset with ID {} not found", id))
        }
    }

    fn select_item(&self, id: &AssetID) -> Result<&AssetContainer, String> {
        self.registry
            .get(id)
            .ok_or_else(|| format!("Asset with ID {} not found", id))
    }

    fn query_load_inner(&self, qid: AssetQueryID, aid: AssetID) -> Result<AssetQueryID, String> {
        let factory = self.select_factory(&aid)?;
        let item = self.select_item(&aid)?;
        match &item.state {
            AssetState::Raw(raw) => {
                // If the asset is raw, we need to load it
                let message = InMessage::Load(qid, aid.clone(), raw.clone(), item.header.clone());
                let res = Self::send_message(factory, message);
                if !res.is_ok() {
                    return Err(format!("Failed to send load message for asset {}", aid));
                }
            }
            _ => {
                // If the asset is already loaded or freed, we can return an error
                return Err(format!("Asset {} is already loaded or freed", aid));
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
    ) -> Result<AssetQueryID, String> {
        let factory = self.select_factory(&aid)?;
        let item = self.select_item(&aid)?;
        match &item.state {
            AssetState::Loaded(_, _) => {
                if item.rc.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                    // If the asset is still in use, we cannot free it
                    return Err(format!("Asset {} is currently in use", aid));
                }

                // If the asset is loaded, we can free it
                let message = InMessage::Free(qid, aid.clone());
                if !Self::send_message(factory, message).is_ok() {
                    return Err(format!("Failed to send free message for asset {}", aid));
                }
            }
            _ => {
                // If the asset is not loaded, we can return an error
                return Err(format!("Asset {} is not loaded", aid));
            }
        }

        debug!("Query {} sent for asset {}", qid, aid);
        self.queries.add_query(qid.clone());
        Ok(qid)
    }

    fn query_load(&self, id: AssetID) -> Result<AssetQueryID, String> {
        let qid = AssetQueryID::new();
        self.query_load_inner(qid, id)
    }

    pub fn query_load_all(&self) -> Result<AssetQueryID, String> {
        // let qid = AssetQueryID::new();
        // TODO: Batch queries is not implemented yet.
        for id in self.registry.keys() {
            self.query_load_inner(AssetQueryID::new(), id.clone())?;
        }
        Ok(AssetQueryID::new())
    }

    pub fn query_free(&self, id: AssetID) -> Result<AssetQueryID, String> {
        let qid = AssetQueryID::new();
        self.query_free_inner(qid, id)
    }

    pub fn query_free_all(&mut self) -> Result<AssetQueryID, String> {
        // let qid = AssetQueryID::new();
        // TODO: Batch queries is not implemented yet.
        for id in self.registry.keys() {
            let qid = AssetQueryID::new();
            self.query_free_inner(qid.clone(), id.clone())?;
        }
        Ok(AssetQueryID::new())
    }

    pub fn get(&self, id: AssetID) -> Option<Asset> {
        if let Some(item) = self.registry.get(&id) {
            match &item.state {
                AssetState::Loaded(tid, ptr) => {
                    let rc = Arc::clone(&item.rc);
                    let asset = Asset::new(tid.clone(), rc, ptr.clone());
                    Some(asset)
                }
                AssetState::Freed => None,
                AssetState::Raw(_) => None, // Raw Assets are not accessible
            }
        } else {
            warn!("Asset with ID {} not found", id);
            None
        }
    }

    /// Moves the asset hub into the ECS world.
    /// This will allow automatically processing async events from the asset factories.
    /// This also will provide additional ECS events as `AssetHubEvent` that can be used to
    /// track assets loading and freeing.
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
