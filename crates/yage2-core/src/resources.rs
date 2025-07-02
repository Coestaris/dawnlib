use crate::resources::ResourceData::Unloaded;
use dashmap::mapref::one::Ref;
use dashmap::{DashMap, Entry};
use log::{info, warn};
use std::any::Any;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::rc::Rc;
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

pub struct ResourceManagerConfig {
    pub backend: Box<dyn ResourceManagerIO>,
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

pub enum ResourceRaw {
    // Raw data loaded from the IO-backend
    Dropped,

    // Raw data that has been loaded from the IO-backend
    Loaded(Vec<u8>),
}

#[derive(Clone)]
pub struct Resource(Arc<dyn Any + Send + Sync>);

impl Resource {
    pub fn new<T: Any + Send + Sync>(data: T) -> Self {
        Resource(Arc::new(data))
    }

    pub fn as_any(&self) -> &dyn Any {
        &*self.0
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

pub enum ResourceData {
    // No data loaded
    Unloaded,

    // Data that has been parsed and is ready for use
    Parsed(Resource),
}

pub type ParserFn = fn(&ResourceMetadata, &[u8]) -> Result<Resource, String>;
pub type FinalizerFn = fn(&ResourceMetadata, &Resource) -> Result<(), String>;

pub struct ResourceFactory {
    // The type of the resource this handler is responsible for
    resource_type: ResourceType,

    // Performs the parsing and processing the resource data
    // into the usable format, such as a texture, audio buffer, etc.
    // After the parsing is done, the resource is transformed into a Parsed state
    parser: ParserFn,

    // Frees all the resources allocated by the parser
    // After the finalizer is called, the resource is transformed into a Unloaded state
    finalizer: FinalizerFn,

    // Indicates whether the resource has been updated since the last load
    has_updates: AtomicBool,
}

impl ResourceFactory {
    pub fn finalize_all() {}

    pub fn poll() {}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceMetadata {
    pub name: String,
    pub tag: ResourceTag,
    pub id: ResourceId,
    pub resource_type: ResourceType,
    pub checksum: ResourceChecksum,
}

struct ResourceContainer {
    // Metadata about the resource
    metadata: RwLock<ResourceMetadata>,

    // Raw data loaded from the IO-backend.
    raw: RwLock<ResourceRaw>,

    // Data containing the resource
    data: RwLock<ResourceData>,

    fresh: AtomicBool,
}

pub struct ResourceManager {
    backend: Mutex<Box<dyn ResourceManagerIO>>,
    resources: DashMap<ResourceType, DashMap<ResourceId, ResourceContainer>>,
    factories: DashMap<ResourceType, Arc<ResourceFactory>>,
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

#[derive(Debug)]
pub enum ResourceGetError {
    NotFound,
    NoFactory,
    NoRawDataLoaded,
    ParserFailed(String),
}

impl std::fmt::Display for ResourceGetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceGetError::NotFound => write!(f, "Resource not found"),
            ResourceGetError::NoFactory => {
                write!(f, "No factory registered for this resource type")
            }
            ResourceGetError::NoRawDataLoaded => write!(f, "No raw data loaded for this resource"),
            ResourceGetError::ParserFailed(msg) => write!(f, "Parser failed: {}", msg),
        }
    }
}
impl std::error::Error for ResourceGetError {}

impl ResourceManager {
    pub fn new(config: ResourceManagerConfig) -> Self {
        ResourceManager {
            backend: Mutex::new(config.backend),
            resources: DashMap::new(),
            factories: DashMap::new(),
        }
    }

    // Registers a new resource handler for a specific resource type
    // The handler is responsible for parsing the resource data and finalizing it
    pub fn register_factory(
        &self,
        resource_type: ResourceType,
        parser: ParserFn,
        finalizer: FinalizerFn,
    ) -> Arc<ResourceFactory> {
        info!("Registering resource factory for type: {:?}", resource_type);
        let factory = ResourceFactory {
            resource_type,
            parser,
            finalizer,
            has_updates: AtomicBool::new(false),
        };

        let factory = Arc::new(factory);
        self.factories.insert(resource_type, factory.clone());
        factory
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

        /* Just add them to the list for now */
        if let Some(resources) = updated_resources {
            for (id, metadata) in resources {
                let type_map = self.resources.entry(metadata.resource_type).or_default();
                let entry = type_map.entry(id).or_insert_with(|| {
                    info!("Adding new resource: {:?}", metadata);
                    ResourceContainer {
                        metadata: RwLock::new(metadata.clone()),
                        raw: RwLock::new(ResourceRaw::Dropped),
                        data: RwLock::new(ResourceData::Unloaded),
                        fresh: AtomicBool::new(false),
                    }
                });

                // Update metadata if it has changed
                let mut existing_metadata = entry.metadata.write().unwrap();
                if &*existing_metadata != &metadata {
                    info!("Updating resource metadata: {:?}", metadata);
                    *existing_metadata = metadata;
                    entry.fresh.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn load_all(&self) {
        let mut backend = self.backend.lock().unwrap();
        for type_map in self.resources.iter() {
            for resource_container in type_map.value().iter() {
                if resource_container.fresh.load(Ordering::Relaxed) {
                    continue; // Already fresh, skip loading
                }

                let id = resource_container.key();
                match backend.load(*id) {
                    Ok(raw_data) => {
                        let mut raw = resource_container.raw.write().unwrap();
                        *raw = ResourceRaw::Loaded(raw_data);
                        resource_container.fresh.store(true, Ordering::Relaxed);
                    }
                    Err(e) => {
                        warn!("Failed to load resource {}: {}", id, e);
                    }
                }
            }
        }
    }

    pub fn get_resource(
        &self,
        resource_type: ResourceType,
        resource_id: ResourceId,
    ) -> Result<Resource, ResourceGetError> {
        let type_map = self
            .resources
            .get(&resource_type)
            .ok_or(ResourceGetError::NotFound)?;
        let resource = type_map
            .get(&resource_id)
            .ok_or(ResourceGetError::NotFound)?;

        let mut data = resource.data.write().unwrap();
        let raw = resource.raw.read().unwrap();
        let metadata = resource.metadata.read().unwrap();
        if let ResourceData::Parsed(ref resource) = *data {
            return Ok(resource.clone());
        }

        if let ResourceRaw::Loaded(ref raw_data) = *raw {
            let factory = match self.factories.get(&resource_type) {
                Some(factory) => factory,
                None => return Err(ResourceGetError::NoFactory),
            };

            // Parse the raw data into a usable resource
            let parsed_resource = (factory.parser)(&metadata, raw_data)
                .map_err(|e| ResourceGetError::ParserFailed(e))?;

            // Update the resource data to Parsed state
            *data = ResourceData::Parsed(parsed_resource.clone());

            // Mark the resource as fresh
            resource.fresh.store(true, Ordering::Relaxed);

            Ok(parsed_resource)
        } else {
            Err(ResourceGetError::NoRawDataLoaded)
        }
    }
}
