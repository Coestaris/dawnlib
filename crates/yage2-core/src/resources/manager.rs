use crate::resources::factory::{Factory, InMessage, OutMessage};
use crate::resources::reader::{ResourceHeader, ResourceReader};
use crate::resources::resource::{Resource, ResourceID, ResourceType};
use crossbeam_queue::ArrayQueue;
use std::collections::HashMap;
use crate::resources::r#ref::ResourceRef;

struct FactoryStorage {
    in_queue: ArrayQueue<InMessage>,
    out_queue: ArrayQueue<OutMessage>,
    factory: Box<dyn Factory>,
}

struct ResourceManager<R: ResourceReader> {
    reader: R,
    factories: HashMap<ResourceType, FactoryStorage>,
    registry: HashMap<ResourceID, ResourceHeader>,
}

impl<R: ResourceReader> ResourceManager<R> {
    fn new(reader: R) -> Self {
        ResourceManager {
            reader,
            factories: HashMap::new(),
            registry: HashMap::new(),
        }
    }

    fn register_factory<F: Factory + 'static>(&mut self, resource_type: ResourceType, factory: F) {
        self.factories.insert(
            resource_type,
            FactoryStorage {
                in_queue: ArrayQueue::new(100),  // Example size, adjust as needed
                out_queue: ArrayQueue::new(100), // Example size, adjust as needed
                factory: Box::new(factory),
            },
        );
    }

    fn get_resource(&self, id: &ResourceID) -> Result<Resource, String> {
        let header = self.registry.get(id).unwrap();
        let factory_storage = self.factories.get(&header.resource_type).unwrap();
        
        factory_storage.in_queue.push(InMessage::Load(id.clone()));
        // Assuming the factory processes the queue and returns a resource
        match factory_storage.out_queue.pop() {
            
            _ => Err(format!("Failed to load resource: {}", id)),
        }
    }
}
