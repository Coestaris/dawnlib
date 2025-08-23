use crate::manifest::Manifest;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ReadError {
    IoError(std::io::Error),
    ReadTarError(std::io::Error),
    DecodeError(String),
    ParseAssetError(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::IoError(e) => write!(f, "I/O error: {}", e),
            ReadError::ReadTarError(e) => write!(f, "Failed to read tar entry: {}", e),
            ReadError::DecodeError(e) => write!(f, "Failed to decode contents: {}", e),
            ReadError::ParseAssetError(e) => write!(f, "Failed to parse asset: {}", e),
        }
    }
}

pub fn read_manifest(path: PathBuf) -> Result<Manifest, ReadError> {
    todo!()
}

pub fn read(path: PathBuf, id: AssetID) -> Result<IRAsset, ReadError> {
    todo!()
}
