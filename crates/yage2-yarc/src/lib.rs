mod reader;
mod writer;

pub use reader::read;
use serde::{Deserialize, Serialize};
use std::time::Instant;
pub use writer::write_from_directory;
pub use writer::WriterError;
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::AssetHeader;

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

fn serialize_instant<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // TODO: Implement serialization
    Ok(serializer.serialize_str(&instant.elapsed().as_secs().to_string())?)
}

fn deserialize_instant<'de, D>(deserializer: D) -> Result<Instant, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // TODO: Implement deserialization
    Ok(Instant::now())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    // File information
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    #[serde(
        serialize_with = "serialize_instant",
        deserialize_with = "deserialize_instant"
    )]
    pub date_created: Instant,

    // Technical information
    pub tool: String,
    pub tool_version: String,
    pub serializer: String,
    pub serializer_version: String,
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub headers: Vec<AssetHeader>,
}

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

impl Manifest {
    fn new(write_options: &WriteOptions, headers: Vec<AssetHeader>) -> Self {
        Manifest {
            tool: "yage2-yarc".to_string(),
            tool_version: "0.1.0".to_string(), // TODO: Get from Cargo.toml
            date_created: Instant::now(),
            serializer: asset_serialize::get_tool_name(),
            serializer_version: asset_serialize::get_tool_version(),
            compression: write_options.compression,
            read_mode: write_options.read_mode,
            checksum_algorithm: write_options.checksum_algorithm,
            author: write_options.author.clone(),
            description: write_options.description.clone(),
            license: write_options.license.clone(),
            version: write_options.version.clone(),
            headers,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        let string =
            toml::to_string(self).map_err(|e| format!("Failed to serialize Manifest: {}", e))?;
        Ok(string.into_bytes())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, String> {
        let string = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("Failed to deserialize Manifest: {}", e))?;
        Ok(
            toml::from_str(&string)
                .map_err(|e| format!("Failed to deserialize Manifest: {}", e))?,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PackedAsset {
    pub header: AssetHeader,
    pub raw: AssetRaw,
}

impl PackedAsset {
    pub fn new(header: AssetHeader, raw: AssetRaw) -> Self {
        Self { header, raw }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        asset_serialize::serialize(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, String> {
        asset_serialize::deserialize(bytes)
    }
}
