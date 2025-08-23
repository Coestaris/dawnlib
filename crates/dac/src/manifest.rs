use crate::writer::{ChecksumAlgorithm, ReadMode, WriteConfig};
use dawn_assets::AssetHeader;
use log::warn;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub headers: Vec<AssetHeader>,
}
impl Manifest {
    fn generator_tool() -> String {
        "dawn-dac".to_string() // TODO: Get from Cargo.toml
    }

    pub fn generator_tool_version() -> String {
        "0.1.0".to_string() // TODO: Get from Cargo.toml
    }

    pub fn location() -> &'static str {
        "_manifest"
    }

    pub(crate) fn new(write_options: &WriteConfig, headers: Vec<AssetHeader>) -> Self {
        Manifest {
            tool: Self::generator_tool(),
            tool_version: Self::generator_tool_version(),
            created: SystemTime::now(),
            read_mode: write_options.read_mode,
            checksum_algorithm: write_options.checksum_algorithm,
            author: write_options.author.clone(),
            description: write_options.description.clone(),
            license: write_options.license.clone(),
            version: write_options.version.clone(),
            headers,
        }
    }

    pub fn validate(manifest: &Manifest) -> Result<(), String> {
        // Validate the manifest
        if manifest.tool_version != Self::generator_tool_version() {
            warn!(
                "Manifest tool version mismatch: expected {}, got {}",
                Self::generator_tool_version(),
                manifest.tool_version
            );
        }

        Ok(())
    }
}
