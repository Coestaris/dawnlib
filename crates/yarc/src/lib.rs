mod manifest;
mod reader;
mod writer;

use dawn_assets::ir::IRAsset;
use dawn_assets::AssetHeader;
pub use manifest::Manifest;
pub use reader::read;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
pub use writer::write_from_directory;
pub use writer::WriterError;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Compression {
    None,
    Gzip,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    Md5,
    Blake3,
}

impl Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Md5 => write!(f, "MD5"),
            Self::Blake3 => write!(f, "Blake3"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteOptions {
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,

    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
}

#[cfg(any())]
mod asset_serialize {
    use crate::PackedAsset;

    pub fn get_tool_name() -> String {
        "toml_parser".to_string() // TODO: Get from Cargo.toml
    }
    pub fn get_tool_version() -> String {
        "0.1.0".to_string() // TODO: Get from Cargo.toml
    }

    pub fn serialize(asset: &PackedAsset) -> Result<Vec<u8>, String> {
        toml::to_string(asset)
            .map_err(|e| format!("Failed to serialize PackedAsset: {}", e))
            .and_then(|s| Ok(s.into_bytes()))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<PackedAsset, String> {
        toml::from_slice(bytes).map_err(|e| format!("Failed to deserialize PackedAsset: {}", e))
    }
}

#[cfg(all())]
mod asset_serialize {
    use crate::PackedAsset;

    pub fn get_tool_name() -> String {
        "rmp_serde".to_string() // TODO: Get from Cargo.toml
    }
    pub fn get_tool_version() -> String {
        "0.1.0".to_string() // TODO: Get from Cargo.toml
    }

    pub fn serialize(asset: &PackedAsset) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(asset).map_err(|e| format!("Failed to serialize AssetRaw: {}", e))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<PackedAsset, String> {
        rmp_serde::from_slice(bytes).map_err(|e| format!("Failed to deserialize AssetRaw: {}", e))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PackedAsset {
    pub header: AssetHeader,
    pub ir: IRAsset,
}

impl PackedAsset {
    pub fn new(header: AssetHeader, ir: IRAsset) -> Self {
        Self { header, ir }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        asset_serialize::serialize(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, String> {
        asset_serialize::deserialize(bytes)
    }
}
