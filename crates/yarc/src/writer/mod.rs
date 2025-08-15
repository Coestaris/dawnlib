mod raw;
mod user;
mod pix;

use crate::manifest::Manifest;
use crate::writer::raw::user_asset_to_raw;
use crate::writer::user::UserAsset;
use crate::{ChecksumAlgorithm, Compression, PackedAsset, ReadMode, WriteOptions};
use flate2::write::GzEncoder;
use log::info;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::Builder;
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::{AssetHeader, AssetID};

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

/// Normalize the file name by removing the extension, converting to lowercase,
/// replacing whitespace with underscores, and removing special characters.
fn normalize_name<P: AsRef<std::path::Path>>(path: P) -> String {
    // Get rid of the extension and normalize the name
    let name = path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_lowercase();

    // Replace whitespace with underscores and remove special characters
    name.replace('.', "_")
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "")
}

fn collect_user_assets(
    files: &[PathBuf],
    options: WriteOptions,
) -> Result<Vec<(UserAsset, PathBuf)>, WriterError> {
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
                user_assets.push((asset, toml_file.clone()));
            }

            Err(e) => {
                Err(WriterError::ParseMetadataFailed(toml_file.clone(), e))?;
            }
        }
    }

    Ok(user_assets)
}

fn user_assets_to_raws(
    user_asses: Vec<(UserAsset, PathBuf)>,
    checksum_algorithm: ChecksumAlgorithm,
) -> Result<Vec<(AssetHeader, AssetRaw, PathBuf)>, WriterError> {
    let mut result = Vec::new();
    for (asset, path) in user_asses {
        info!("Processing asset: {}", asset.header.id);
        let (header, raw) = user_asset_to_raw(path.as_path(), &asset, checksum_algorithm)
            .map_err(|e| WriterError::ValidationFailed(e))?;

        result.push((header, raw, path.clone()));
    }
    Ok(result)
}

fn raws_to_binary(
    raws: Vec<(AssetHeader, AssetRaw, PathBuf)>,
    manifest: &Manifest,
) -> Result<Vec<(Vec<u8>, PathBuf)>, WriterError> {
    let mut binary = Vec::new();

    // Serialize the manifest
    let serialized = manifest
        .serialize()
        .map_err(WriterError::SerializationError)?;
    binary.push((serialized, PathBuf::from(Manifest::location())));

    // Serialize each raw asset
    for (header, raw, path) in raws {
        let packed_asset = PackedAsset::new(header, raw);
        let serialized = packed_asset
            .serialize()
            .map_err(WriterError::SerializationError)?;
        binary.push((serialized, path));
    }

    Ok(binary)
}

fn check_dependencies(raws: &[(AssetHeader, AssetRaw, PathBuf)]) -> Result<(), WriterError> {
    for (header, _, _) in raws {
        for dep in &header.dependencies {
            if !raws.iter().any(|(h, _, _)| &h.id == dep) {
                return Err(WriterError::DependenciesMissing(
                    header.id.clone(),
                    dep.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn add_binaries<W>(tar: &mut Builder<W>, raws: &[(Vec<u8>, PathBuf)]) -> Result<(), WriterError>
where
    W: std::io::Write,
{
    for (raw, path) in raws {
        let normalized_name = normalize_name(path);
        let mut header = tar::Header::new_gnu();
        header
            .set_path(normalized_name)
            .map_err(WriterError::TarError)?;
        header.set_size(raw.len() as u64);
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

        tar.append(&header, raw.as_slice())
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
    let raws = user_assets_to_raws(user_assets, options.checksum_algorithm)?;
    check_dependencies(&raws)?;
    let manifest = Manifest::new(&options, raws.iter().map(|(h, _, _)| h.clone()).collect());
    let binaries = raws_to_binary(raws, &manifest)?;

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
