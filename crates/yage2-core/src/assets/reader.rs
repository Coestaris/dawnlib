use crate::assets::{AssetID, AssetType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetChecksum([u8; 16]);

impl AssetChecksum {
    pub fn from_bytes(bytes: &[u8]) -> AssetChecksum {
        let mut checksum = [0; 16];
        let len = bytes.len().min(16);
        checksum[..len].copy_from_slice(&bytes[..len]);
        AssetChecksum(checksum)
    }
}

impl Default for AssetChecksum {
    fn default() -> Self {
        AssetChecksum([0; 16])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetHeader {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub asset_type: AssetType,
    #[serde(default)]
    pub checksum: AssetChecksum,
}

impl Default for AssetHeader {
    fn default() -> Self {
        AssetHeader {
            name: String::new(),
            tags: Vec::new(),
            asset_type: AssetType::Unknown,
            checksum: AssetChecksum::default(),
        }
    }
}

pub trait AssetReader {
    fn has_updates(&self) -> bool;
    fn enumerate(&mut self) -> Result<HashMap<AssetID, AssetHeader>, String>;
    fn load(&mut self, id: AssetID) -> Result<Vec<u8>, String>;
}
