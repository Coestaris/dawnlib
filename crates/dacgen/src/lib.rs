mod cache;
pub mod config;
mod deep_hash;
mod ir;
mod source;
mod user;

use crate::cache::Cache;
use crate::config::WriteConfig;
use crate::deep_hash::{DeepHash, DeepHashCtx};
use crate::user::UserAsset;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetChecksum, AssetHeader, AssetID};
use dawn_dac::serialize_backend::serialize;
use dawn_dac::writer::{write_container, BinaryAsset};
use dawn_dac::{
    ChecksumAlgorithm, CompressionLevel, CompressionMode, ContainerError, Manifest, ReadMode,
    Version,
};
use dawn_util::profile::Measure;
use log::{debug, info};
use rayon::prelude::*;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;
use thiserror::Error;

build_info::build_info!(fn build_info);

fn generator_tool() -> String {
    let bi = build_info();
    bi.crate_info.name.clone()
}

fn generator_tool_version() -> Version {
    let bi = build_info();
    Version::new(
        bi.crate_info.version.major as u16,
        bi.crate_info.version.minor as u16,
        bi.crate_info.version.patch as u16,
        None,
    )
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
pub(crate) struct UserAssetFile {
    asset: UserAsset,
    path: PathBuf,
}

impl DeepHash for UserAssetFile {
    fn deep_hash<T: Hasher>(&self, state: &mut T, ctx: &mut DeepHashCtx) -> anyhow::Result<()> {
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
    fn convert(&self, compression_level: CompressionLevel) -> Result<BinaryAsset, WriterError> {
        let _measure = Measure::new(format!(
            "Compressed {}",
            self.header.id.clone().as_str().to_string()
        ));

        let serialized = serialize(&self.ir).map_err(|e| WriterError::SerializationError(e))?;

        // Not worth compressing such small files
        if serialized.len() > 256 {
            let compressed =
                dawn_dac::compression_backend::compress(&serialized, compression_level)
                    .map_err(|e| WriterError::CompressionError(e))?;

            // Check if the compression was effective
            if compressed.len() != 0 && compressed.len() < serialized.len() {
                return Ok(BinaryAsset {
                    raw: compressed,
                    compression: CompressionMode::Brotli,
                    header: self.header.clone(),
                });
            }
        }

        Ok(BinaryAsset {
            raw: serialized,
            compression: CompressionMode::None,
            header: self.header.clone(),
        })
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
    #[error("Hash failed: {0}")]
    HashError(anyhow::Error),
    #[error("Failed to serialize: {0}")]
    SerializationError(anyhow::Error),
    #[error("Failed to compress data: {0}")]
    CompressionError(anyhow::Error),
    #[error("Failed to validate metadata: {0}")]
    ConvertingToIRFailed(PathBuf, anyhow::Error),
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
    config: WriteConfig,
) -> Result<(), WriterError> {
    let input_files = collect_files(input_dir.clone(), config.read_mode)?;

    let cache = Cache::new(
        config.clone(),
        config.cache_dir.clone(),
        input_dir.clone(),
        config.checksum_algorithm,
    );
    let user_assets = collect_user_assets(&input_files)?;

    debug!("Converting User Assets");
    let binaries = user_assets
        .par_iter()
        .map(|user_asset| {
            if let Some(cached) = cache.get(&user_asset) {
                Ok(cached)
            } else {
                let user_clone = user_asset.clone();

                let instant = std::time::Instant::now();
                let irs = user_asset
                    .convert(
                        config.cache_dir.as_path(),
                        input_dir.as_path(),
                        config.checksum_algorithm.clone(),
                    )
                    .map_err(|e| WriterError::ConvertingToIRFailed(user_asset.path.clone(), e))?;
                debug!("Converted {:?} in {:?}", user_asset.path, instant.elapsed());

                let binaries = irs
                    .par_iter()
                    .map(|ir| ir.convert(config.compression_level.clone()))
                    .collect::<Result<Vec<BinaryAsset>, WriterError>>()?;

                cache.insert(&user_clone, &binaries)?;
                Ok(binaries)
            }
        })
        .collect::<Result<Vec<Vec<BinaryAsset>>, WriterError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<BinaryAsset>>();

    debug!("Collected {} binaries", binaries.len());
    let headers = binaries
        .iter()
        .map(|b| b.header.clone())
        .collect::<Vec<_>>();

    sanity_check(&headers)?;

    let manifest = create_manifest(&config, headers);

    info!("Creating DAC container");
    write_container(writer, manifest, binaries)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{write_from_directory, WriteConfig};
    use dawn_dac::reader::read_manifest;
    use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode, Version};

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

        // I'll deal with it later
        #[cfg(unix)]
        mod dirs {
            // TODO: Do not commit me :(
            pub const CURRENT_DIR: &str = "/home/taris/work/dawn/assets";
            pub const OUTPUT_FILE: &str = "/tmp/cache/assets.dac";
            pub const CACHE_DIR: &str = "/tmp/cache";
        }
        #[cfg(windows)]
        mod dirs {
            // TODO: Do not commit me :(
            pub const CURRENT_DIR: &str = r"D:\coding\dawn\assets";
            pub const OUTPUT_FILE: &str = r"D:\coding\cache\output.dac";
            pub const CACHE_DIR: &str = r"D:\coding\cache\";
        }

        let file = std::fs::File::create(dirs::OUTPUT_FILE).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        write_from_directory(
            &mut writer,
            dirs::CURRENT_DIR.into(),
            WriteConfig {
                read_mode: ReadMode::Recursive,
                checksum_algorithm: ChecksumAlgorithm::Blake3,
                compression_level: CompressionLevel::None,
                cache_dir: dirs::CACHE_DIR.into(),
                author: Some("Coestaris <vk_vm@ukr.net>".to_string()),
                description: Some("Test assets".to_string()),
                version: Some(Version::new(1, 0, 0, None)),
                license: Some("MIT".to_string()),
            },
        )
        .unwrap();
        drop(writer.into_inner().unwrap());

        let file = std::fs::File::open(dirs::OUTPUT_FILE).unwrap();
        let mut reader = std::io::BufReader::new(file);
        let manifest = read_manifest(&mut reader).unwrap();
        // println!("{:#?}", manifest);
        // manifest.tree("sponza".into(), &|id, header, depth| {
        //     println!(
        //         "{}- {} (deps: {})",
        //         "  ".repeat(depth),
        //         id.as_str(),
        //         header.dependencies.len()
        //     );
        // });

        // let ir = read_asset(&mut reader, "barrel".into()).unwrap();
        // println!("{:#?}", ir);
    }
}
