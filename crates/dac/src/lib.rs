use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub mod container;
pub mod manifest;
pub mod reader;
pub mod writer;

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
    use std::io::{Read, Write};

    pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
        // TODO: Make compression level configurable
        let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 2, 22);
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
