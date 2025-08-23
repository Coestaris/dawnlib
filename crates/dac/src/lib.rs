use serde::{Deserialize, Serialize};
use std::fmt::Display;

mod layout;
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
    use serde::{Deserialize, Serialize};

    pub fn serialize<T: Serialize>(object: &T) -> Result<Vec<u8>, String> {
        bincode::encode_to_vec(object, bincode::config::standard()).map_err(|e| e.to_string())
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, String> {
        bincode::decode_from_slice(bytes, bincode::config::standard())
            .map(|(obj, _)| obj)
            .map_err(|e| e.to_string())
    }
}
