use crate::resources::resource::{ResourceID, ResourceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceChecksum([u8; 16]);

impl ResourceChecksum {
    pub fn from_bytes(bytes: &[u8]) -> ResourceChecksum {
        let mut checksum = [0; 16];
        let len = bytes.len().min(16);
        checksum[..len].copy_from_slice(&bytes[..len]);
        ResourceChecksum(checksum)
    }
}

impl Default for ResourceChecksum {
    fn default() -> Self {
        ResourceChecksum([0; 16])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceHeader {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub resource_type: ResourceType,
    #[serde(default)]
    pub checksum: ResourceChecksum,
}

impl Default for ResourceHeader {
    fn default() -> Self {
        ResourceHeader {
            name: String::new(),
            tags: Vec::new(),
            resource_type: ResourceType::Unknown,
            checksum: ResourceChecksum::default(),
        }
    }
}

pub trait ResourceReader {
    fn has_updates(&self) -> bool;
    fn enumerate_resources(&mut self) -> Result<HashMap<ResourceID, ResourceHeader>, String>;
    fn load(&mut self, id: ResourceID) -> Result<Vec<u8>, String>;
}
