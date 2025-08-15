use crate::{asset_serialize, ChecksumAlgorithm, Compression, ReadMode, WriteOptions};
use log::warn;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime};
use yage2_core::assets::AssetHeader;

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    // File information
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,

    // Technical information
    pub tool: String,
    pub tool_version: String,
    pub created: SystemTime,
    pub serializer: String,
    pub serializer_version: String,
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub headers: Vec<AssetHeader>,
}
impl Manifest {
    fn generator_tool() -> String {
        "yage2-yarc".to_string() // TODO: Get from Cargo.toml
    }

    pub fn generator_tool_version() -> String {
        "0.1.0".to_string() // TODO: Get from Cargo.toml
    }

    pub fn location() -> &'static str {
        "_manifest"
    }

    pub(crate) fn new(write_options: &WriteOptions, headers: Vec<AssetHeader>) -> Self {
        Manifest {
            tool: Self::generator_tool(),
            tool_version: Self::generator_tool_version(),
            created: SystemTime::now(),
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

        let manifest: Manifest = toml::from_str(&string)
            .map_err(|e| format!("Failed to deserialize Manifest: {}", e))?;

        // Validate the manifest
        if manifest.tool_version != Self::generator_tool_version() {
            warn!(
                "Manifest tool version mismatch: expected {}, got {}",
                Self::generator_tool_version(),
                manifest.tool_version
            );
        }
        if manifest.serializer != asset_serialize::get_tool_name() {
            return Err(format!(
                "Manifest serializer mismatch: expected {}, got {}",
                asset_serialize::get_tool_name(),
                manifest.serializer
            ));
        }
        if manifest.serializer_version != asset_serialize::get_tool_version() {
            warn!(
                "Manifest serializer version mismatch: expected {}, got {}",
                asset_serialize::get_tool_version(),
                manifest.serializer_version
            );
        }

        Ok(manifest)
    }
}
