mod ir;
mod pix;
mod user;

use crate::manifest::Manifest;
use crate::writer::ir::user_to_ir;
use crate::writer::user::UserAsset;
use crate::{ChecksumAlgorithm, Compression, PackedAsset, ReadMode, WriteOptions};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetHeader, AssetID};
use flate2::write::GzEncoder;
use log::info;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Builder;

pub(crate) struct UserAssetFile {
    asset: UserAsset,
    path: PathBuf,
}

pub(crate) struct UserIRAsset {
    header: AssetHeader,
    ir: IRAsset,
}

pub(crate) struct AssetBinary {
    raw: Vec<u8>,
    name: String,
}

#[derive(Debug)]
pub enum WriterError {
    CollectingFilesFailed(std::io::Error),
    IoError(std::io::Error),
    TarError(std::io::Error),
    ParseMetadataFailed(PathBuf, toml::de::Error),
    ValidationFailed(String),
    SerializationError(String),
    UnsupportedChecksumAlgorithm(ChecksumAlgorithm),
    DependenciesMissing(AssetID, AssetID),
}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriterError::CollectingFilesFailed(e) => write!(f, "Failed to collect files: {}", e),
            WriterError::IoError(e) => write!(f, "I/O error: {}", e),
            WriterError::TarError(e) => write!(f, "Tar error: {}", e),
            WriterError::ParseMetadataFailed(name, e) => {
                write!(
                    f,
                    "Failed to parse metadata for '{}': {}",
                    name.to_str().unwrap(),
                    e
                )
            }
            WriterError::UnsupportedChecksumAlgorithm(a) => {
                write!(f, "Unsupported checksum algorithm: {:?}", a)
            }
            WriterError::ValidationFailed(msg) => {
                write!(f, "Validation failed: {}", msg)
            }
            WriterError::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
            WriterError::DependenciesMissing(id, dep) => {
                write!(f, "Dependencies missing. {} requires {}", id, dep)
            }
        }
    }
}

impl std::error::Error for WriterError {}

/// Collect files from the specified path based on the read mode
/// and return a vector of file paths.
fn collect_files<P: AsRef<std::path::Path>>(
    path: P,
    read_mode: ReadMode,
) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
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

fn collect_user_assets(
    files: &[PathBuf],
    options: WriteOptions,
) -> Result<Vec<UserAssetFile>, WriterError> {
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
        let mut file = File::open(toml_file).map_err(WriterError::IoError)?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(WriterError::IoError)?;

        // Parse the metadata
        match toml::from_str::<UserAsset>(&content) {
            Ok(asset) => {
                user_assets.push(UserAssetFile {
                    asset,
                    path: toml_file.clone(),
                });
            }

            Err(e) => {
                Err(WriterError::ParseMetadataFailed(toml_file.clone(), e))?;
            }
        }
    }

    Ok(user_assets)
}

fn user_assets_to_irs(
    files: Vec<UserAssetFile>,
    checksum_algorithm: ChecksumAlgorithm,
) -> Result<Vec<UserIRAsset>, WriterError> {
    let mut result = Vec::new();
    for file in files {
        result.extend(
            user_to_ir(file, checksum_algorithm).map_err(|e| WriterError::ValidationFailed(e))?,
        );
    }
    Ok(result)
}

fn irs_to_binaries(
    irs: Vec<UserIRAsset>,
    manifest: &Manifest,
) -> Result<Vec<AssetBinary>, WriterError> {
    let mut binary = Vec::new();

    // Serialize the manifest
    let serialized = manifest
        .serialize()
        .map_err(WriterError::SerializationError)?;
    binary.push(AssetBinary {
        raw: serialized,
        name: Manifest::location().to_string(),
    });

    // Serialize each IR asset
    for ir in irs {
        let name = ir.header.id.to_string();
        let packed_asset = PackedAsset::new(ir.header, ir.ir);
        let serialized = packed_asset
            .serialize()
            .map_err(WriterError::SerializationError)?;
        binary.push(AssetBinary {
            raw: serialized,
            name,
        });
    }

    Ok(binary)
}

fn check_dependencies(irs: &[UserIRAsset]) -> Result<(), WriterError> {
    for ir in irs {
        for dep in &ir.header.dependencies {
            if !irs.iter().any(|i| i.header.id == *dep) {
                return Err(WriterError::DependenciesMissing(
                    ir.header.id.clone(),
                    dep.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn add_binaries<W>(tar: &mut Builder<W>, binaries: &[AssetBinary]) -> Result<(), WriterError>
where
    W: std::io::Write,
{
    for binary in binaries {
        let mut header = tar::Header::new_gnu();
        header
            .set_path(binary.name.as_str())
            .map_err(WriterError::TarError)?;
        header.set_size(binary.raw.len() as u64);
        header.set_mode(0o644); // Set file mode to 644
        header.set_uid(0); // Set user ID to 0 (root)
        header.set_gid(0); // Set group ID to 0 (root)
        header.set_mtime(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        header.set_cksum();

        tar.append(&header, binary.raw.as_slice())
            .map_err(WriterError::TarError)?;
    }
    Ok(())
}

/// Implementation of creating a YARC from a directory
/// This will involve reading files, normalizing names, and writing to a
/// .tar or .tar.gz archive with the specified compression and checksum algorithm.
pub fn write_from_directory(
    input_dir: PathBuf,
    options: WriteOptions,
    output: PathBuf,
) -> Result<(), WriterError> {
    let input_files =
        collect_files(input_dir, options.read_mode).map_err(WriterError::CollectingFilesFailed)?;
    let user_assets = collect_user_assets(&input_files, options.clone())?;
    let irs = user_assets_to_irs(user_assets, options.checksum_algorithm)?;
    check_dependencies(&irs)?;
    let manifest = Manifest::new(&options, irs.iter().map(|h| h.header.clone()).collect());
    let binaries = irs_to_binaries(irs, &manifest)?;

    info!(
        "Writing {} files to {}",
        binaries.len(),
        output.to_str().unwrap()
    );

    let output_file = File::create(&output).map_err(WriterError::IoError)?;
    match options.compression {
        Compression::None => {
            // Create a tar archive
            let mut tar = Builder::new(output_file);
            add_binaries(&mut tar, &binaries)?;
            tar.finish().map_err(WriterError::TarError)
        }
        Compression::Gzip => {
            // Create a gzipped tar archive
            let enc = GzEncoder::new(output_file, flate2::Compression::default());
            let mut tar = Builder::new(enc);
            add_binaries(&mut tar, &binaries)?;
            tar.finish().map_err(WriterError::TarError)?;
            let mut enc = tar.into_inner().map_err(WriterError::TarError)?;
            enc.finish().map_err(WriterError::IoError)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{write_from_directory, ChecksumAlgorithm, Compression, ReadMode, WriteOptions};
    use std::path::PathBuf;

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
        let current_dir = "/home/taris/work/dawn/assets";
        let target_dir = "/tmp/test.tar.gz";
        write_from_directory(
            current_dir.into(),
            WriteOptions {
                compression: Compression::Gzip,
                read_mode: ReadMode::Recursive,
                checksum_algorithm: ChecksumAlgorithm::Md5,
                author: Some("Coestaris <vk_vm@ukr.net>".to_string()),
                description: Some("Test assets".to_string()),
                version: Some("0.1.0".to_string()),
                license: Some("MIT".to_string()),
            },
            target_dir.into(),
        )
        .unwrap();
    }
}
