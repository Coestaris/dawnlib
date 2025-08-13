use crate::PackedAsset;
use flate2::bufread::GzDecoder;
use log::info;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tar::Archive;
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::AssetHeader;

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

fn read_internal<R>(reader: R) -> Result<Vec<(AssetHeader, AssetRaw)>, ReadError>
where
    R: Read,
{
    let mut archive = Archive::new(reader);

    let mut assets: Vec<(AssetHeader, AssetRaw)> = Vec::new();
    for entry in archive.entries().map_err(ReadError::ReadTarError)? {
        let mut entry = entry.map_err(ReadError::ReadTarError)?;
        let path = entry.path().map_err(ReadError::IoError)?;

        // Manifest is not actually needed for reading assets,
        // but we still read it to ensure compatibility with the format.
        if path == std::path::Path::new(".manifest.toml") {
            continue;
        }

        // Read file and try to deserialize it
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(ReadError::ReadTarError)?;

        let asset = PackedAsset::deserialize(&contents).map_err(ReadError::DecodeError)?;
        assets.push((asset.header, asset.raw));
    }

    Ok(assets)
}

pub fn read_from_file_compressed(path: &Path) -> Result<Vec<(AssetHeader, AssetRaw)>, ReadError> {
    let file = File::open(path).unwrap();
    let buf_reader = BufReader::new(file);
    let decoder = GzDecoder::new(buf_reader);

    read_internal(decoder)
}

pub fn read_from_file_uncompressed(path: &Path) -> Result<Vec<(AssetHeader, AssetRaw)>, ReadError> {
    let file = File::open(path).unwrap();
    let buf_reader = BufReader::new(file);

    read_internal(buf_reader)
}

pub fn read(path: PathBuf) -> Result<Vec<(AssetHeader, AssetRaw)>, ReadError> {
    match read_from_file_compressed(path.as_path()) {
        Err(ReadError::ReadTarError(e)) => {
            // Try to read as non-compressed tar
            info!(
                "Failed to read as compressed tar, trying uncompressed: {}",
                e
            );
            read_from_file_uncompressed(path.as_path())
        }
        any => any,
    }
}
