use crate::assets::factory::{FactoryBinding, InMessage, OutMessage, QueryID};
use crate::assets::reader::ResourceReader;
use crate::assets::registry::{
    QueriesRegistry, ResourceRegistryItem, ResourceState, ResourcesRegistry,
};
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
pub enum ResourceEvent {
    QueryCompleted(QueryID),
    ResourceLoaded(AssetID),
    ResourceFreed(AssetID),
    AllResourcesLoaded,
    AllResourcesFreed,
}

#[derive(Component)]
pub struct ResourceManager {
    factories: HashMap<AssetType, FactoryBinding>,
    out_queue: Arc<ArrayQueue<OutMessage>>,
    registry: ResourcesRegistry,
    queries: QueriesRegistry,
}

impl ResourceManager {
    pub fn new<R: ResourceReader>(mut reader: R) -> Result<Self, String> {
        let mut registry = ResourcesRegistry::new();
        for (item_id, header) in reader.enumerate_resources()? {
            let raw = reader.load(item_id.clone())?;
            registry.push(item_id.clone(), raw, header);
        }

        Ok(ResourceManager {
            factories: HashMap::new(),
            out_queue: Arc::new(ArrayQueue::new(100)), // Example size, adjust as needed
            registry,
            queries: QueriesRegistry::new(),
        })
    }

    pub fn create_factory_biding(&mut self, resource_type: AssetType) -> FactoryBinding {
        if self.factories.contains_key(&resource_type) {
            panic!(
                "Factory for resource type {:?} already registered",
                resource_type
            );
        }

        // Setup async queues for the factory
        let in_queue = Arc::new(ArrayQueue::new(100)); // Example size, adjust as needed
        let out_queue = &self.out_queue;
        let binding =
            FactoryBinding::new(resource_type, Arc::clone(&in_queue), Arc::clone(out_queue));

        info!(
            "Creating factory binding for resource type {:?}",
            resource_type
        );
        self.factories.insert(resource_type, binding.clone());
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
            if let Some(factory) = self.factories.get(&item.header.resource_type) {
                Ok(factory)
            } else {
                Err(format!(
                    "No factory registered for resource type {:?}",
                    item.header.resource_type
                ))
            }
        } else {
            Err(format!("Resource with ID {} not found", id))
        }
    }

    fn select_item(&self, id: &AssetID) -> Result<&ResourceRegistryItem, String> {
        self.registry
            .get(id)
            .ok_or_else(|| format!("Resource with ID {} not found", id))
    }

    fn query_load_inner(&self, qid: QueryID, id: AssetID) -> Result<QueryID, String> {
        let factory = self.select_factory(&id)?;
        let item = self.select_item(&id)?;
        match &item.state {
            ResourceState::Raw(raw) => {
                // If the resource is raw, we need to load it
                let message =
                    InMessage::Load(QueryID::new(), id.clone(), raw.clone(), item.header.clone());
                let res = Self::send_message(factory, message);
                if !res.is_ok() {
                    return Err(format!("Failed to send load message for resource {}", id));
                }
            }
            _ => {
                // If the resource is already loaded or freed, we can return an error
                return Err(format!("Resource {} is already loaded or freed", id));
            }
        }

        debug!("Query {} sent for resource {}", qid, id);
        self.queries.add_query(qid.clone());
        Ok(qid)
    }

    pub fn query_free_inner(&self, qid: QueryID, id: AssetID) -> Result<QueryID, String> {
        let factory = self.select_factory(&id)?;
        let item = self.select_item(&id)?;
        match &item.state {
            ResourceState::Loaded(_) => {
                if item.in_use.load(std::sync::atomic::Ordering::SeqCst) {
                    // If the resource is in use, we cannot free it
                    return Err(format!("Resource {} is currently in use", id));
                }

                // If the resource is loaded, we can free it
                let message = InMessage::Free(qid, id.clone());
                if !Self::send_message(factory, message).is_ok() {
                    return Err(format!("Failed to send free message for resource {}", id));
                }
            }
            _ => {
                // If the resource is not loaded, we can return an error
                return Err(format!("Resource {} is not loaded", id));
            }
        }

        debug!("Query {} sent for resource {}", qid, id);
        self.queries.add_query(qid.clone());
        Ok(qid)
    }

    fn query_load(&self, id: AssetID) -> Result<QueryID, String> {
        let qid = QueryID::new();
        self.query_load_inner(qid, id)
    }

    pub fn query_load_all(&self) -> Result<QueryID, String> {
        let qid = QueryID::new();
        for id in self.registry.keys() {
            self.query_load_inner(qid.clone(), id.clone())?;
        }
        Ok(qid)
    }

    pub fn query_free(&self, id: AssetID) -> Result<QueryID, String> {
        let qid = QueryID::new();
        self.query_free_inner(qid, id)
    }

    pub fn query_free_all(&mut self) -> Result<QueryID, String> {
        let qid = QueryID::new();
        for id in self.registry.keys() {
            self.query_free_inner(qid.clone(), id.clone())?;
        }
        Ok(qid)
    }

    pub fn get_resource(&self, id: AssetID) -> Option<Asset> {
        if let Some(item) = self.registry.get(&id) {
            match &item.state {
                ResourceState::Loaded(ptr) => {
                    let in_use = item.in_use.clone();
                    in_use.store(true, std::sync::atomic::Ordering::SeqCst);
                    let resource = Asset::new(in_use, ptr.cast::<()>());
                    Some(resource)
                }
                ResourceState::Freed => None,
                ResourceState::Raw(_) => None, // Raw resources are not accessible
            }
        } else {
            warn!("Resource with ID {} not found", id);
            None
        }
    }

    /// Moves the resource manager into the ECS world.
    /// This will allow automatically processing async events from the resource factories.
    /// This also will provide additional ECS events as `ResourceEvent` that can be used to
    /// track resource loading and freeing.
    pub fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);

        fn tick_handler(
            _: Receiver<Tick>,
            mut manager: Single<&mut ResourceManager>,
            mut sender: Sender<ResourceEvent>,
        ) {
            while let Some(message) = manager.out_queue.pop() {
                match message {
                    OutMessage::Loaded(qid, res_id, ptr) => {
                        debug!("Query {} loaded resource {}", qid, res_id);
                        if let Some(item) = manager.registry.get_mut(&res_id) {
                            item.state = ResourceState::Loaded(ptr);
                        } else {
                            warn!(
                                "Received Loaded message for unknown resource ID: {}",
                                res_id
                            );
                            continue;
                        }

                        sender.send(ResourceEvent::ResourceLoaded(res_id.clone()));
                        sender.send(ResourceEvent::QueryCompleted(qid.clone()));
                        manager.queries.remove_query(&qid);

                        if manager.registry.all_loaded() {
                            info!("All resources loaded");
                            sender.send(ResourceEvent::AllResourcesLoaded);
                        }
                    }

                    OutMessage::Freed(qid, res_id) => {
                        debug!("Query {} freed resource {}", qid, res_id);
                        if let Some(item) = manager.registry.get_mut(&res_id) {
                            item.state = ResourceState::Freed;
                        } else {
                            warn!("Received Freed message for unknown resource ID: {}", res_id);
                        }

                        sender.send(ResourceEvent::ResourceFreed(res_id.clone()));
                        sender.send(ResourceEvent::QueryCompleted(qid.clone()));
                        manager.queries.remove_query(&qid);

                        if manager.registry.all_freed() {
                            info!("All resources loaded");
                            sender.send(ResourceEvent::AllResourcesFreed);
                        }
                    }
                }
            }
        }

        world.add_handler(tick_handler);
    }
}
