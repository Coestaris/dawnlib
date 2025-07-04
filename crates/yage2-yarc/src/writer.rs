use crate::structures::{
    Compression, HashAlgorithm, ReadMode, ResourceMetadata, TypeSpecificMetadata, YARCManifest,
    YARCWriteOptions,
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use yage2_core::resources::{ResourceChecksum, ResourceHeader, ResourceType};
use yage2_core::utils::format_now;

#[derive(Debug)]
struct Container {
    binary_path: std::path::PathBuf,
    resource_type: ResourceType,
    metadata_path: std::path::PathBuf,
    metadata: ResourceMetadata,
}

#[derive(Debug)]
pub enum WriterError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
    Other(String),
}

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

/// Preprocessor function type that takes a file path and metadata,
/// and returns a processed file path or an error.
type PreProcessor<'a> = fn(
    &'a std::path::PathBuf,
    &ResourceMetadata,
) -> Result<(&'a std::path::PathBuf, ResourceMetadata), WriterError>;

fn dummy_preprocessor<'a>(
    path: &'a std::path::PathBuf,
    metadata: &ResourceMetadata,
) -> Result<(&'a std::path::PathBuf, ResourceMetadata), WriterError> {
    // This is a dummy preprocessor that does nothing
    // In a real implementation, this could be used to preprocess files
    // (e.g., compiling shaders, converting audio formats, etc.)
    Ok((path, metadata.clone()))
}

fn extension_to_resource_type(ext: &str) -> (ResourceType, PreProcessor) {
    match ext {
        // Shader types
        "glsl" => (ResourceType::ShaderGLSL, dummy_preprocessor),
        "spv" => (ResourceType::ShaderSPIRV, dummy_preprocessor),
        "hlsl" => (ResourceType::ShaderHLSL, dummy_preprocessor),

        // Audio types
        "flac" => (ResourceType::AudioFLAC, dummy_preprocessor),
        "wav" => (ResourceType::AudioWAV, dummy_preprocessor),
        "ogg" => (ResourceType::AudioOGG, dummy_preprocessor),
        "mp3" => (ResourceType::AudioMP3, dummy_preprocessor),

        // Image types
        "png" => (ResourceType::ImagePNG, dummy_preprocessor),
        "jpg" | "jpeg" => (ResourceType::ImageJPEG, dummy_preprocessor),
        "bmp" => (ResourceType::ImageBMP, dummy_preprocessor),

        // Font types
        "ttf" => (ResourceType::FontTTF, dummy_preprocessor),
        "otf" => (ResourceType::FontOTF, dummy_preprocessor),

        // Model types
        "obj" => (ResourceType::ModelOBJ, dummy_preprocessor),
        "fbx" => (ResourceType::ModelFBX, dummy_preprocessor),
        "gltf" | "glb" => (ResourceType::ModelGLTF, dummy_preprocessor),

        _ => (ResourceType::Unknown, dummy_preprocessor),
    }
}

fn checksum<P: AsRef<std::path::Path>>(
    path: P,
    algorithm: HashAlgorithm,
) -> Result<ResourceChecksum, WriterError> {
    use std::fs::File;
    use std::io::Read;

    let content = File::open(path)
        .map_err(WriterError::IoError)?
        .bytes()
        .collect::<Result<Vec<u8>, std::io::Error>>()
        .map_err(WriterError::IoError)?;
    let hash = match algorithm {
        HashAlgorithm::Md5 => {
            let mut hasher = md5::Context::new();
            hasher.consume(&content);
            hasher.compute().0
        }
        _ => panic!("Unimplemented hash algorithm"),
    };

    // Convert the hash to a u64 (this is a simplification, real
    // hash functions may produce larger hashes)
    // TODO: Handle larger hashes properly
    Ok(u64::from_le_bytes(hash[0..8].try_into().unwrap_or([0; 8])))
}

fn create_manifest(create_options: YARCWriteOptions, containers: &[Container]) -> YARCManifest {
    let mut manifest_resource = Vec::new();
    for resource in containers {
        manifest_resource.push(resource.metadata.common.clone());
    }

    YARCManifest {
        tool_created: "Yage2 Packager".to_string(),
        tool_version: "0.1.0".to_string(), // TODO: Get from Cargo.toml
        date_created: format_now().unwrap(),
        write_options: create_options,
        resources: manifest_resource,
    }
}

fn validate_metadata<P: AsRef<std::path::Path>>(
    metadata_path: P,
    resource_type: ResourceType,
    name: &str,
) -> Result<ResourceMetadata, WriterError> {
    let metadata_content =
        std::fs::read_to_string(metadata_path.as_ref()).map_err(WriterError::IoError)?;
    let mut metadata: ResourceMetadata =
        toml::from_str(&metadata_content).map_err(WriterError::ParseError)?;

    // If some fields are missing, the serde will fill them with defaults
    // But we need to ensure that the resource type matches
    if metadata.common.resource_type == ResourceType::Unknown {
        metadata.common.resource_type = resource_type;
    } else if metadata.common.resource_type != resource_type {
        return Err(WriterError::Other(format!(
            "Resource type mismatch: expected {:?}, found {:?}",
            resource_type, metadata.common.resource_type
        )));
    }

    // Ensure the type-specific metadata is set
    match metadata.type_specific {
        TypeSpecificMetadata::Unknown => {
            metadata.type_specific = TypeSpecificMetadata::default_for(resource_type);
        }
        _ => {
            // Ensure the type-specific metadata is suitable for the resource type
            if !metadata.type_specific.suitable_for(resource_type) {
                return Err(WriterError::Other(format!(
                    "Type-specific metadata {:?} is not suitable for resource type {:?}",
                    metadata.type_specific, resource_type
                )));
            }
        }
    };

    // If name is empty, use the provided name
    if metadata.common.name.is_empty() {
        metadata.common.name = name.to_string();
    }

    Ok(metadata)
}

fn create_metadata<P: AsRef<std::path::Path>>(
    resource_type: ResourceType,
    name: &str,
) -> Result<ResourceMetadata, WriterError> {
    let common_metadata = ResourceHeader {
        name: name.to_string(),
        tag: String::new(),
        resource_type,
        checksum: 0,
    };

    Ok(ResourceMetadata {
        common: common_metadata,
        type_specific: TypeSpecificMetadata::default_for(resource_type),
    })
}

// Split the file path into the name and extension
fn split_path(path: &std::path::Path) -> (String, String) {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    (name, ext)
}

fn prepare_files<P: AsRef<std::path::Path>>(
    input_files: &Vec<std::path::PathBuf>,
    options: &YARCWriteOptions,
    directory: &TempDir,
) -> Result<Vec<std::path::PathBuf>, WriterError> {
    let mut resources = Vec::new();
    for mut file in input_files {
        let (we, ext) = split_path(&file);

        if ext == "toml" {
            // Skip metadata files
            debug!("Skipping metadata file: {}", we);
            continue;
        }

        let mut name = normalize_name(&file);
        let (resource_type, preprocessor) = extension_to_resource_type(&ext);
        info!("Processing file: {} (type: {:?})", we, resource_type);

        let toml = file.with_extension("toml");
        let mut metadata = if toml.exists() {
            // Metadata object found, validate it
            validate_metadata(&toml, resource_type, &name).unwrap()
        } else {
            // If file not exist. Create new one
            create_metadata::<P>(resource_type, &name).unwrap()
        };

        // Preprocess file here, since it can modify the metadata
        (file, metadata) = preprocessor(file, &metadata).unwrap();
        // If user/preprocessor changed the name, we need to update it
        name = metadata.common.name.clone();
        // Update the metadata with the checksum
        metadata.common.checksum = checksum(&file, options.hash_algorithm)?;
        // Write the metadata to a temporary file
        let metadata_path = directory.path().join(&name).with_extension("toml");
        let metadata_content = toml::to_string(&metadata).unwrap();
        std::fs::write(&metadata_path, metadata_content).unwrap();

        // Copy the file to the temp directory
        let dest_path = directory.path().join(&name);
        debug!("Copying file {} to {}", we, dest_path.display());
        std::fs::copy(file, &dest_path).map_err(WriterError::IoError)?;

        // Create the resource entry
        resources.push(Container {
            binary_path: dest_path,
            resource_type,
            metadata_path,
            metadata,
        });
    }

    // Create the manifest
    let manifest = create_manifest(options.clone(), &resources);

    // Write the manifest to a temporary file
    let manifest_path = directory.path().join(".manifest.toml");
    let manifest_content = toml::to_string(&manifest).unwrap();
    std::fs::write(&manifest_path, manifest_content).map_err(WriterError::IoError)?;

    // Make sure that manifest is the first file in the archive
    let mut resources_with_manifest = vec![manifest_path];
    for resource in resources {
        resources_with_manifest.push(resource.binary_path);
        // Also include the metadata file
        resources_with_manifest.push(resource.metadata_path);
    }
    info!(
        "Created manifest with {} resources",
        resources_with_manifest.len()
    );
    Ok(resources_with_manifest)
}

fn add_files<W>(
    tar_builder: &mut tar::Builder<W>,
    files: &[std::path::PathBuf],
) -> Result<(), WriterError>
where
    W: std::io::Write,
{
    for resource in files {
        let mut file = std::fs::File::open(resource).map_err(WriterError::IoError)?;
        tar_builder
            .append_file(resource.file_name().unwrap(), &mut file)
            .map_err(WriterError::IoError)?;
    }

    Ok(())
}

/// Implementation of creating a YARC from a directory
/// This will involve reading files, normalizing names, and writing to a .tar or .tar.gz archive
/// with the specified compression and hash algorithm.
pub fn write_from_directory<P: AsRef<std::path::Path>>(
    input_dir: P,
    options: YARCWriteOptions,
    output: P,
) -> Result<(), WriterError> {
    // Read the directory and collect files based on the read mode
    let files = collect_files(input_dir, options.read_mode).map_err(WriterError::IoError)?;

    // Group files with their metadata
    let temp_dir = TempDir::new("yage2_yarc").map_err(WriterError::IoError)?;
    let output_files = prepare_files::<P>(&files, &options.clone(), &temp_dir)?;

    // Create the output archive
    let output_path = output.as_ref();
    let output_file = std::fs::File::create(output_path).map_err(WriterError::IoError)?;
    match options.compression {
        Compression::None => {
            // Create a tar archive
            let mut tar = tar::Builder::new(output_file);
            add_files(&mut tar, &output_files)?;
            tar.finish().map_err(WriterError::IoError)?;
        }
        Compression::Gzip => {
            // Create a gzipped tar archive
            let enc = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);
            add_files(&mut tar, &output_files)?;
            tar.finish().map_err(WriterError::IoError)?;
        }
    }

    Ok(())
}
