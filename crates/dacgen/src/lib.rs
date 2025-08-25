mod cache;
mod deep_hash;
mod ir;
mod source;
mod user;

use crate::cache::Cache;
use crate::deep_hash::{DeepHash, DeepHashCtx};
use crate::user::UserAsset;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader, AssetID};
use dawn_dac::container::writer::{write_container, BinaryAsset};
use dawn_dac::container::{CompressionMode, ContainerError};
use dawn_dac::serialize_backend::serialize;
use dawn_dac::{ChecksumAlgorithm, CompressionLevel, Manifest, ReadMode};
use log::{debug, info};
use std::fs::File;
use std::hash::Hasher;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;

fn generator_tool() -> String {
    "dawn-dac".to_string() // TODO: Get from Cargo.toml
}

fn generator_tool_version() -> String {
    "0.1.0".to_string() // TODO: Get from Cargo.toml
}

pub(crate) fn create_manifest(write_options: &WriteConfig, headers: Vec<AssetHeader>) -> Manifest {
    Manifest {
        tool: generator_tool(),
        tool_version: generator_tool_version(),
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

#[derive(Debug, Clone)]
pub struct WriteConfig {
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub compression_level: CompressionLevel,
    pub cache_dir: PathBuf,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UserAssetFile {
    asset: UserAsset,
    path: PathBuf,
}

impl DeepHash for UserAssetFile {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> Result<(), String> {
        self.asset.deep_hash(state, ctx)?;
        // We do not hash the path, as it is not relevant to the content
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct UserIRAsset {
    header: AssetHeader,
    ir: IRAsset,
}

impl UserIRAsset {
    fn convert(self, compression_level: CompressionLevel) -> Result<BinaryAsset, WriterError> {
        debug!("Converting {:?} into Binary", self);

        let serialized =
            serialize(&self.ir).map_err(|e| WriterError::SerializationError(e.to_string()))?;
        let compressed = dawn_dac::compression_backend::compress(&serialized, compression_level)
            .map_err(|e| WriterError::CompressionError(e))?;

        // Check if the compression was effective
        Ok(
            if compressed.len() != 0 && compressed.len() < serialized.len() {
                BinaryAsset {
                    raw: compressed,
                    compression: CompressionMode::None,
                    header: self.header,
                }
            } else {
                BinaryAsset {
                    raw: serialized,
                    compression: CompressionMode::None,
                    header: self.header,
                }
            },
        )
    }
}

#[derive(Debug, Error)]
pub enum WriterError {
    #[error("IO-related error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unsupported compression: {0}")]
    UnsupportedChecksumAlgorithm(ChecksumAlgorithm),
    #[error("Failed to parse metadata: {0}: {1}")]
    DeserializationError(PathBuf, toml::de::Error),
    #[error("Failed to serialize: {0}")]
    SerializationError(String),
    #[error("Failed to compress data: {0}")]
    CompressionError(String),
    #[error("Failed to validate metadata: {0}")]
    ConvertingToIRFailed(String),
    #[error("Unsupported read mode: {0}")]
    DependenciesMissing(AssetID, AssetID),
    #[error("Circular dependency detected: {0} -> {1}")]
    CircleDependency(AssetID, AssetID),
    #[error("Non-unique ID: {0}")]
    NonUniqueID(AssetID),
    #[error("Container creation failed: {0}")]
    ContainerCreationFailed(#[from] ContainerError),
}

/// Collect files from the specified path based on the read mode
/// and return a vector of file paths.
fn collect_files(path: PathBuf, read_mode: ReadMode) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    match read_mode {
        ReadMode::Flat => {
            // Collect files in flat mode
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    files.push(entry.path());
                }
            }
        }
        ReadMode::Recursive => {
            // Collect files recursively
            for entry in walkdir::WalkDir::new(path) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    files.push(entry.into_path());
                }
            }
        }
    };

    Ok(files)
}

fn collect_user_assets(files: &[PathBuf]) -> Result<Vec<UserAssetFile>, WriterError> {
    // Find all toml files
    let mut toml_files = Vec::new();
    for file in files {
        if file.extension().and_then(|e| e.to_str()) == Some("toml") {
            toml_files.push(file.clone());
        }
    }

    // Read toml files
    let mut user_assets = Vec::new();
    for toml_file in &toml_files {
        let mut file = File::open(toml_file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Parse the metadata
        match toml::from_str::<UserAsset>(&content) {
            Ok(asset) => {
                user_assets.push(UserAssetFile {
                    asset,
                    path: toml_file.clone(),
                });
            }

            Err(e) => {
                Err(WriterError::DeserializationError(toml_file.clone(), e))?;
            }
        }
    }

    Ok(user_assets)
}

fn sanity_check(headers: &[AssetHeader]) -> Result<(), WriterError> {
    // Check that all dependencies are present
    for header in headers {
        for dep in &header.dependencies {
            if !headers.iter().any(|i| i.id == *dep) {
                return Err(WriterError::DependenciesMissing(
                    header.id.clone(),
                    dep.clone(),
                ));
            }
        }
    }

    // Check that there's no circular dependencies
    for header_a in headers {
        for header_b in headers {
            if header_a.id != header_b.id && header_a.dependencies.contains(&header_b.id) {
                if header_b.dependencies.contains(&header_a.id) {
                    return Err(WriterError::CircleDependency(
                        header_a.id.clone(),
                        header_b.id.clone(),
                    ));
                }
            }
        }
    }

    // Check that all IDs are unique
    let mut ids = std::collections::HashSet::new();
    for ir in headers {
        if !ids.insert(ir.id.clone()) {
            return Err(WriterError::NonUniqueID(ir.id.clone()));
        }
    }

    Ok(())
}

pub fn write_from_directory<W: Write>(
    writer: &mut W,
    input_dir: PathBuf,
    options: WriteConfig,
) -> Result<(), WriterError> {
    let input_files = collect_files(input_dir.clone(), options.read_mode)?;

    let cache = Cache::new(
        options.cache_dir.clone(),
        input_dir.clone(),
        options.checksum_algorithm,
    );
    let mut binaries = Vec::new();
    for user_asset in collect_user_assets(&input_files)? {
        if let Some(cached) = cache.get(&user_asset) {
            binaries.extend(cached);
        } else {
            let mut file_binaries = Vec::new();
            let user_clone = user_asset.clone();
            for ir in user_asset
                .convert(
                    options.cache_dir.as_path(),
                    input_dir.as_path(),
                    options.checksum_algorithm.clone(),
                )
                .map_err(|e| WriterError::ConvertingToIRFailed(e))?
            {
                file_binaries.push(ir.convert(options.compression_level.clone())?);
            }

            cache.insert(&user_clone, &file_binaries)?;
            binaries.extend(file_binaries);
        }
    }

    let headers = binaries
        .iter()
        .map(|b| b.header.clone())
        .collect::<Vec<_>>();
    sanity_check(&headers)?;

    let manifest = create_manifest(&options, headers);

    info!("Creating DAC container");
    write_container(writer, manifest, binaries)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{write_from_directory, WriteConfig};
    use dawn_dac::reader::{read_asset, read_manifest};
    use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode};

    #[test]
    fn test() {
        // Setup basic logging
        struct Logger;
        impl log::Log for Logger {
            fn enabled(&self, metadata: &log::Metadata) -> bool {
                metadata.level() <= log::Level::Debug
            }

            fn log(&self, record: &log::Record) {
                println!("{}: {}", record.level(), record.args());
            }

            fn flush(&self) {}
        }

        log::set_logger(&Logger).unwrap();
        log::set_max_level(log::LevelFilter::Debug);

        // TODO: Do not commit me :(
        let current_dir = r"D:\coding\dawn\assets";
        let target_dir = r"D:\coding\output.dac";
        let cache_dir = r"D:\coding\cache";
        let file = std::fs::File::create(target_dir).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        write_from_directory(
            &mut writer,
            current_dir.into(),
            WriteConfig {
                read_mode: ReadMode::Recursive,
                checksum_algorithm: ChecksumAlgorithm::Blake3,
                compression_level: CompressionLevel::None,
                cache_dir: cache_dir.into(),
                author: Some("Coestaris <vk_vm@ukr.net>".to_string()),
                description: Some("Test assets".to_string()),
                version: Some("0.1.0".to_string()),
                license: Some("MIT".to_string()),
            },
        )
        .unwrap();
        drop(writer.into_inner().unwrap());

        let file = std::fs::File::open(target_dir).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let manifest = read_manifest(&mut reader).unwrap();
        println!("{:#?}", manifest);
        let ir = read_asset(&mut reader, "sponza".into()).unwrap();
        println!("{:#?}", ir);
    }
}
