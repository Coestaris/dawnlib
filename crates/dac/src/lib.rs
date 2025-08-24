use dawn_assets::AssetHeader;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::time::SystemTime;

pub mod container;
pub mod reader;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    Blake3,
    Md5,
    SHA256,
}

impl Display for ChecksumAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blake3 => write!(f, "Blake3"),
            Self::Md5 => write!(f, "MD5"),
            Self::SHA256 => write!(f, "SHA256"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompressionLevel {
    None,
    UltraFast,
    Fast,
    Balanced,
    Best,
}

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

#[cfg(any())]
pub mod serialize_backend {
    use serde::{Deserialize, Serialize};
    use toml;

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        toml::to_string(object)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string())
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
        toml::from_slice(bytes).map_err(|e| e.to_string())
    }
}

#[cfg(all())]
pub mod serialize_backend {
    use bincode;
    use serde::de::DeserializeOwned;
    use serde::Serialize;

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        bincode::serde::encode_to_vec(object, bincode::config::standard())
            .map_err(|e| e.to_string())
    }

    pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
        bincode::serde::decode_from_slice(bytes, bincode::config::standard())
            .map(|(obj, _)| obj)
            .map_err(|e| e.to_string())
    }
}

pub mod compression_backend {
    use crate::CompressionLevel;
    use std::io::{Read, Write};

    pub fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>, String> {
        let level = match level {
            CompressionLevel::None => return Ok(data.to_vec()),
            CompressionLevel::UltraFast => 1,
            CompressionLevel::Fast => 4,
            CompressionLevel::Balanced => 7,
            CompressionLevel::Best => 11,
        };

        let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, level, 22);
        encoder.write_all(data).map_err(|e| e.to_string())?;
        encoder.flush().map_err(|e| e.to_string())?;
        Ok(encoder.into_inner())
    }

    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
        let mut decompressed = Vec::new();
        let mut reader = brotli::Decompressor::new(data, 4096);
        reader
            .read_to_end(&mut decompressed)
            .map_err(|e| e.to_string())?;
        Ok(decompressed)
    }
}
