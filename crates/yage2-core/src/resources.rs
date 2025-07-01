use crate::resources::ResourceData::Unloaded;
use dashmap::mapref::one::Ref;
use dashmap::{DashMap, Entry};
use log::{info, warn};
use std::any::Any;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

pub type ResourceId = usize;
pub type ResourceTag = usize;
pub type ResourceChecksum = u64;

pub trait ResourceManagerIO {
    fn has_updates(&self) -> bool;
    fn enumerate_resources(&self) -> HashMap<ResourceId, ResourceMetadata>;
    fn load(&mut self, id: ResourceId) -> Result<Vec<u8>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadStrategy {
    // Load the resource immediately, blocking the thread until it's loaded
    Lazy,

    // If possible, load the resource immediately
    OnPoll(ResourceSelector),

    // Load the resources only when explicitly requested
    Manual,
}

pub struct ResourceManagerConfig {
    pub backend: Box<dyn ResourceManagerIO>,
    pub load_strategy: LoadStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Texture,
    Shader,
    Mesh,
    Audio,
    Font,
    Script,
}

pub enum ResourceData {
    // No data loaded
    Unloaded,

    // Data that has been parsed and is ready for use
    Parsed(Arc<dyn Any + Send + Sync>),
}

// Performs the parsing and processing the resource data
// into the usable format, such as a texture, audio buffer, etc.
// After the parsing is done, the resource is transformed into a Parsed state
pub type ResourceParser =
    fn(&ResourceMetadata, &[u8]) -> Result<Arc<dyn Any + Send + Sync>, String>;

// Frees all the resources allocated by the parser
// After the finalizer is called, the resource is transformed into a Unloaded state
pub type ResourceFinalizer =
    fn(&ResourceMetadata, &Arc<dyn Any + Send + Sync>) -> Result<(), String>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceMetadata {
    name: String,
    tag: ResourceTag,
    id: ResourceId,
    resource_type: ResourceType,
    checksum: ResourceChecksum,
}

struct Resource {
    // Metadata about the resource
    metadata: RwLock<ResourceMetadata>,

    // Raw data loaded from the IO-backend
    raw: RwLock<Vec<u8>>,
    raw_fresh: AtomicBool,

    // Data containing the resource
    data: RwLock<ResourceData>,
    data_fresh: AtomicBool,
}

pub struct ResourceManager {
    backend: Mutex<Box<dyn ResourceManagerIO>>,
    resources: DashMap<ResourceId, Arc<Resource>>,
    load_strategy: LoadStrategy,
    handler: DashMap<ResourceType, (ResourceParser, ResourceFinalizer)>,
}

impl PartialEq<ResourceMetadata> for &ResourceMetadata {
    fn eq(&self, other: &ResourceMetadata) -> bool {
        self.name == other.name
            && self.tag == other.tag
            && self.id == other.id
            && self.resource_type == other.resource_type
            && self.checksum == other.checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum ResourceSelector {
    Type(ResourceType),
    Tag(ResourceTag),
    Id(ResourceId),
    Any,
}

impl ResourceSelector {
    pub fn matches(&self, metadata: &ResourceMetadata) -> bool {
        match self {
            ResourceSelector::Type(resource_type) => metadata.resource_type == *resource_type,
            ResourceSelector::Tag(tag) => metadata.tag == *tag,
            ResourceSelector::Id(id) => metadata.id == *id,
            ResourceSelector::Any => true,
        }
    }
}

impl ResourceManager {
    pub fn new(config: ResourceManagerConfig) -> Self {
        ResourceManager {
            backend: Mutex::new(config.backend),
            load_strategy: config.load_strategy,
            resources: DashMap::new(),
            handler: DashMap::new(),
        }
    }

    pub fn register_handler(
        &mut self,
        resource_type: ResourceType,
        parser: ResourceParser,
        finalizer: ResourceFinalizer,
    ) {
        self.handler.insert(resource_type, (parser, finalizer));
    }

    pub fn refresh(&self, selector: &ResourceSelector) {
        let mut to_refresh = Vec::new();
        for entry in self.resources.iter() {
            let metadata = entry.metadata.read().unwrap();
            if selector.matches(&metadata) {
                if !entry.data_fresh.load(Ordering::Acquire) {
                    to_refresh.push(Arc::clone(entry.value()));
                }
            }
        }

        if to_refresh.is_empty() {
            return;
        }

        for resource in to_refresh {
            if !resource.raw_fresh.load(Ordering::Acquire) {
                match &self.load_strategy {
                    LoadStrategy::Lazy => {
                        self.load_inner(&resource);
                    }

                    _ => {}
                }
            }

            // If something is parsed, we should finalize it
            let data = resource.data.read().unwrap();
            if let ResourceData::Parsed(parsed) = *data {
                info!(
                    "Finalizing resource {}",
                    resource.metadata.read().unwrap().id
                );
                self.finalize_inner(&resource, &parsed).unwrap();
            }

            // Parse the raw if there's some
            let raw = resource.raw.read().unwrap();
            if !raw.is_empty() {
                info!("Parsing resource {}", resource.metadata.read().unwrap().id);
                self.parse_inner(&resource).unwrap();
            }
        }
    }

    fn parse_inner(&self, resource: &Arc<Resource>) -> Result<(), String> {
        let metadata = resource.metadata.read().unwrap();
        let raw = resource.raw.read().unwrap();
        let handler = self.handler.get(&metadata.resource_type).ok_or_else(|| {
            format!(
                "No handler registered for resource type {:?}",
                metadata.resource_type
            )
        })?;

        let (parser, _) = handler.value();
        let parsed_data = parser(&metadata, &raw)?;
        *resource.data.write().unwrap() = ResourceData::Parsed(parsed_data);
        resource.data_fresh.store(true, Ordering::Release);
        // Clear the raw data after parsing
        resource.raw.write().unwrap().clear();
        Ok(())
    }

    fn finalize_inner(&self, resource: &Arc<Resource>, parsed_data: &Arc<dyn Any + Send + Sync>) -> Result<(), String> {
        let metadata = resource.metadata.read().unwrap();
        let raw = resource.raw.read().unwrap();
        let handler = self.handler.get(&metadata.resource_type).ok_or_else(|| {
            format!(
                "No handler registered for resource type {:?}",
                metadata.resource_type
            )
        })?;

        let (_, finalizer) = handler.value();
        let res = finalizer(&metadata, parsed_data);
        *resource.data.write().unwrap() = Unloaded;
        res
    }

    fn load_inner(&self, resource: &Arc<Resource>) {
        let id = resource.metadata.read().unwrap().id;
        let mut backend = self.backend.lock().unwrap();
        match backend.load(id) {
            Ok(data) => {
                info!("Resource {} loaded", id);
                let mut raw = resource.raw.write().unwrap();
                *raw = data;
                resource.raw_fresh.store(true, Ordering::Release);
            }
            Err(e) => {
                info!("Failed to load resource {}: {}", id, e);
            }
        }
    }

    pub fn load(&self, selector: ResourceSelector) {
        let mut to_load = Vec::new();
        for entry in self.resources.iter() {
            let metadata = entry.metadata.read().unwrap();
            if selector.matches(&metadata) {
                if !entry.raw_fresh.load(Ordering::Acquire) {
                    to_load.push(Arc::clone(entry.value()));
                }
            }
        }

        for resource in to_load {
            self.load_inner(&resource);
        }
    }

    // Checks if the IO backend has any updates
    // If some new resources are available, they are added in the Unloaded state
    // If some resources have been updated, they are marked as unfresh
    // Calling this function will not load any resources
    // It is not recommended to call this function too often
    pub fn poll_io(&self) {
        let updated_resources = {
            let backend = self.backend.lock().unwrap();
            if backend.has_updates() {
                Some(backend.enumerate_resources())
            } else {
                None
            }
        };

        if let Some(resources) = updated_resources {
            for (id, metadata) in resources {
                match self.resources.entry(id) {
                    Entry::Occupied(entry) => {
                        let mut resource_metadata = entry.get().metadata.write().unwrap();
                        if *resource_metadata != metadata {
                            info!("Resource {} metadata changed", id);
                            *resource_metadata = metadata.clone();
                            entry.get().data_fresh.store(false, Ordering::Release);
                            entry.get().raw_fresh.store(false, Ordering::Release);
                        }
                    }

                    Entry::Vacant(entry) => {
                        info!("Adding new resource {}", id);
                        entry.insert(Arc::new(Resource {
                            metadata: RwLock::new(metadata),
                            data: RwLock::new(ResourceData::Unloaded),
                            raw: RwLock::new(Vec::new()),
                            raw_fresh: AtomicBool::new(false),
                            data_fresh: AtomicBool::new(false),
                        }));
                    }
                }
            }
        }

        match self.load_strategy {
            LoadStrategy::Lazy => {
                // Do nothing, resources will be loaded on demand
            }
            LoadStrategy::OnPoll(selector) => {
                self.load(selector);
            }
            LoadStrategy::Manual => {
                // Do nothing, resources will be loaded when explicitly requested
            }
        }
    }
}
