use dashmap::DashMap;
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
    fn enumerate_resources(&self) -> Result<HashMap<ResourceId, ResourceMetadata>, String>;
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
    AudioWAV,
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

pub trait ResourceFactory {
    fn parse(&self, metadata: &ResourceMetadata, raw: &[u8]) -> Result<Resource, String>;

    fn finalize(&self, metadata: &ResourceMetadata, resource: &Resource) -> Result<(), String>;
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
    factories: DashMap<ResourceType, Arc<dyn ResourceFactory>>,
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
pub enum ResourceManagerLoadError {
    IOError(String),
}

impl std::fmt::Display for ResourceManagerLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceManagerLoadError::IOError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for ResourceManagerLoadError {}

#[derive(Debug)]
pub enum ResourceManagerGetError {
    ResourceNotFound,
    NoSuitableFactory,
    NoRawDataLoaded,
    ParserFailed(String),
}

impl std::fmt::Display for ResourceManagerGetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceManagerGetError::ResourceNotFound => write!(f, "Resource not found"),
            ResourceManagerGetError::NoSuitableFactory => {
                write!(f, "No factory registered for this resource type")
            }
            ResourceManagerGetError::NoRawDataLoaded => {
                write!(f, "No raw data loaded for this resource")
            }
            ResourceManagerGetError::ParserFailed(msg) => write!(f, "Parser failed: {}", msg),
        }
    }
}
impl std::error::Error for ResourceManagerGetError {}

#[derive(Debug)]
pub enum ResourceManagerPollError {
    EnumerateFailed(String),
}

impl std::fmt::Display for ResourceManagerPollError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceManagerPollError::EnumerateFailed(msg) => {
                write!(f, "Failed to enumerate resources: {}", msg)
            }
        }
    }
}

impl std::error::Error for ResourceManagerPollError {}

impl ResourceManager {
    pub fn new(config: ResourceManagerConfig) -> Self {
        ResourceManager {
            backend: Mutex::new(config.backend),
            resources: DashMap::new(),
            factories: DashMap::new(),
        }
    }

    pub fn register_factory(&self, resource_type: ResourceType, factory: Arc<dyn ResourceFactory>) {
        info!("Registering resource factory for type: {:?}", resource_type);
        self.factories.insert(resource_type, factory.clone());
    }

    // Checks if the IO backend has any updates
    // If some new resources are available, they are added in the Unloaded state
    // If some resources have been updated, they are marked as unfresh
    // Calling this function will not load any resources
    // It is not recommended to call this function too often
    pub fn poll_io(&self) -> Result<bool, ResourceManagerPollError> {
        let updated_resources = {
            let backend = self.backend.lock().unwrap();
            if backend.has_updates() {
                Some(backend.enumerate_resources())
            } else {
                None
            }
        };

        let updated_resources = match updated_resources {
            Some(Err(e)) => Err(ResourceManagerPollError::EnumerateFailed(e))?,
            Some(Ok(resources)) => resources,
            None => return Ok(false),
        };

        /* Just add them to the list for now */
        let mut any_updates = false;
        for (id, metadata) in updated_resources {
            let type_map = self.resources.entry(metadata.resource_type).or_default();
            let entry = type_map.entry(id).or_insert_with(|| {
                info!("Adding new resource: {:?}", metadata);
                any_updates = true;
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
                any_updates = true;
            }
        }

        Ok(any_updates)
    }

    pub fn load_all(&self) -> Result<(), ResourceManagerLoadError> {
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
                    Err(e) => Err(ResourceManagerLoadError::IOError(e))?,
                }
            }
        }

        Ok(())
    }

    pub fn finalize_all(&self, resource_type: ResourceType) {
        for type_map in self.resources.iter() {
            if type_map.key() != &resource_type {
                continue; // Skip other resource types
            }

            for resource_container in type_map.value().iter() {
                let metadata = resource_container.metadata.read().unwrap();
                let mut data = resource_container.data.write().unwrap();
                if let ResourceData::Parsed(ref resource) = *data {
                    let factory = match self.factories.get(&resource_type) {
                        Some(factory) => factory,
                        None => continue, // No factory registered for this type
                    };

                    // Call the finalizer to clean up resources
                    if let Err(e) = factory.finalize(&metadata, resource) {
                        warn!("Failed to finalize resource {}: {}", metadata.name, e);
                    } else {
                        info!("Finalized resource: {}", metadata.name);
                        // Mark the resource as unloaded
                        *data = ResourceData::Unloaded;
                    }
                }
            }
        }
    }

    pub fn get_resource(
        &self,
        resource_type: ResourceType,
        resource_id: ResourceId,
    ) -> Result<Resource, ResourceManagerGetError> {
        let type_map = self
            .resources
            .get(&resource_type)
            .ok_or(ResourceManagerGetError::ResourceNotFound)?;
        let resource = type_map
            .get(&resource_id)
            .ok_or(ResourceManagerGetError::ResourceNotFound)?;

        let mut data = resource.data.write().unwrap();
        let raw = resource.raw.read().unwrap();
        let metadata = resource.metadata.read().unwrap();
        if let ResourceData::Parsed(ref resource) = *data {
            return Ok(resource.clone());
        }

        if let ResourceRaw::Loaded(ref raw_data) = *raw {
            let factory = match self.factories.get(&resource_type) {
                Some(factory) => factory,
                None => return Err(ResourceManagerGetError::NoSuitableFactory),
            };

            // Parse the raw data into a usable resource
            let parsed_resource = factory
                .parse(&metadata, raw_data)
                .map_err(|e| ResourceManagerGetError::ParserFailed(e))?;

            // Update the resource data to Parsed state
            *data = ResourceData::Parsed(parsed_resource.clone());

            // Mark the resource as fresh
            resource.fresh.store(true, Ordering::Relaxed);

            Ok(parsed_resource)
        } else {
            Err(ResourceManagerGetError::NoRawDataLoaded)
        }
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        // Log about all not finalized resources
        for map in self.resources.iter() {
            for resource_container in map.iter() {
                let metadata = resource_container.metadata.read().unwrap();
                if let ResourceData::Parsed(_) = *resource_container.data.read().unwrap() {
                    warn!(
                        "Resource {} (ID: {}) of type {:?} is not finalized",
                        metadata.name,
                        resource_container.key(),
                        metadata.resource_type
                    );
                }
            }
        }
    }
}
