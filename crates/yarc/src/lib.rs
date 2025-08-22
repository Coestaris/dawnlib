use dawn_assets::ir::IRAsset;
use dawn_assets::AssetHeader;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub mod manifest;
pub mod reader;
pub mod writer;

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
pub mod serialize_backend {
    use crate::PackedAsset;

    pub fn get_tool_name() -> String {
        "toml_parser".to_string() // TODO: Get from Cargo.toml
    }
    pub fn get_tool_version() -> String {
        "0.1.0".to_string() // TODO: Get from Cargo.toml
    }

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {}

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {}
}

#[cfg(all())]
pub mod serialize_backend {
    use serde::{Deserialize, Serialize};
    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec::<T>(object).map_err(|e| format!("Failed to serialize AssetRaw: {}", e))
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
        rmp_serde::from_slice::<'a, T>(bytes)
            .map_err(|e| format!("Failed to deserialize AssetRaw: {}", e))
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
}
