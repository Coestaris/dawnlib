use log::info;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type ResourceId = usize;
type ResourceTag = usize;
type ResourceChecksum = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Texture,
    Shader,
    Mesh,
    Audio,
    Font,
    Script,
}

trait ResourceManagerBackend {
    fn has_updates(&self) -> bool;
    fn enumerate_resources(&self) -> HashMap<ResourceId, ResourceMetadata>;
    fn load(&mut self, id: ResourceId) -> Result<Vec<u8>, String>;
}

pub struct ResourceManagerConfig {
    pub backend: Box<dyn ResourceManagerBackend>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ResourceMetadata {
    name: String,
    id: ResourceId,
    tag: ResourceTag,
    resource_type: ResourceType,
    checksum: ResourceChecksum,
}

struct Resource {
    metadata: ResourceMetadata,
    data: Option<Mutex<Vec<u8>>>,
    is_loaded: bool,
    is_live: bool,
}

pub struct ResourceManager {
    backend: Box<dyn ResourceManagerBackend>,
    resources: HashMap<ResourceId, Resource>,
}

impl PartialEq<ResourceMetadata> for &ResourceMetadata {
    fn eq(&self, other: &ResourceMetadata) -> bool {
        self.name == other.name
            && self.id == other.id
            && self.tag == other.tag
            && self.resource_type == other.resource_type
            && self.checksum == other.checksum
    }
}

impl ResourceManager {
    pub fn new(config: ResourceManagerConfig) -> Self {
        ResourceManager {
            backend: config.backend,
            resources: HashMap::new(),
        }
    }

    pub fn update(&mut self) {
        if !self.backend.has_updates() {
            return;
        }

        // Determine what resource has been updated
        let new_metadata = self.backend.enumerate_resources();
        for (id, metadata) in new_metadata {
            if let Some(existing_resource) = &mut self.resources.get_mut(&id) {
                if existing_resource.metadata != metadata {
                    info!("New version of resource detected: {:?}", metadata);
                    existing_resource.metadata = metadata.clone();
                    existing_resource.is_live = false;
                }
            } else {
                info!("Adding new resource: {:?}", metadata);
                self.resources.insert(
                    id,
                    Resource {
                        metadata: metadata.clone(),
                        data: None,
                        is_loaded: false,
                        is_live: false,
                    },
                );
            }
        }
    }

    fn load_any(&mut self, selector: &impl Fn(&ResourceMetadata) -> bool) {
        for resource in self.resources.values_mut() {
            if selector(&resource.metadata) && !resource.is_loaded {
                match self.backend.load(resource.metadata.id) {
                    Ok(data) => {
                        resource.data = Some(Mutex::new(data));
                        resource.is_loaded = true;
                        resource.is_live = true;
                        info!("Resource loaded: {:?}", resource.metadata);
                    }
                    Err(e) => {
                        info!("Failed to load resource {}: {}", resource.metadata.name, e);
                    }
                }
            }
        }
    }

    pub fn load_type(&mut self, resource_type: ResourceType) {
        self.load_any(&|metadata: &ResourceMetadata| metadata.resource_type == resource_type);
    }

    pub fn load_tag(&mut self, tag: ResourceTag) {
        self.load_any(&|metadata: &ResourceMetadata| metadata.tag == tag);
    }

    pub fn load_id(&mut self, id: ResourceId) {
        self.load_any(&|metadata: &ResourceMetadata| metadata.id == id);
    }
}
