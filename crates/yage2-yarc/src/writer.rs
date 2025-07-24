use crate::preprocessors::{
    compile_glsl_shader, dummy_preprocessor, resample_flac_file, resample_ogg_file,
    resample_wav_file, PreProcessor, PreprocessorsError,
};
use crate::structures::{
    ChecksumAlgorithm, Compression, Manifest, ReadMode, ResourceMetadata, TypeSpecificMetadata,
    WriteOptions,
};
use log::{debug, info};
use std::fs::File;
use std::io::{BufReader, Read};
use tempdir::TempDir;
use yage2_core::resources::{ResourceChecksum, ResourceHeader, ResourceType};
use yage2_core::utils::format_now;

#[derive(Debug)]
struct Container {
    binary_path: std::path::PathBuf,
    metadata_path: std::path::PathBuf,
    metadata: ResourceMetadata,
}

#[derive(Debug)]
pub enum WriterError {
    CollectingFilesFailed(std::io::Error),
    IoError(std::io::Error),
    TarError(std::io::Error),
    ParseMetadataFailed(String, toml::de::Error),
    FormatMetadataFailed(toml::ser::Error),
    ValidateMetadataFailed(String, String),
    PreprocessorFailed(PreprocessorsError),

    UnsupportedCompression(Compression),
    UnsupportedChecksumAlgorithm(ChecksumAlgorithm),
    UnsupportedResourceType(ResourceType),
    UnknownResourceType(String),
}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriterError::CollectingFilesFailed(e) => write!(f, "Failed to collect files: {}", e),
            WriterError::IoError(e) => write!(f, "I/O error: {}", e),
            WriterError::TarError(e) => write!(f, "Tar error: {}", e),
            WriterError::ParseMetadataFailed(name, e) => {
                write!(f, "Failed to parse metadata for '{}': {}", name, e)
            }
            WriterError::FormatMetadataFailed(e) => write!(f, "Failed to format metadata: {}", e),
            WriterError::ValidateMetadataFailed(name, e) => {
                write!(f, "Metadata validation failed for '{}': {}", name, e)
            }
            WriterError::PreprocessorFailed(e) => write!(f, "Preprocessor failed: {}", e),
            WriterError::UnsupportedCompression(c) => write!(f, "Unsupported compression: {:?}", c),
            WriterError::UnsupportedChecksumAlgorithm(a) => {
                write!(f, "Unsupported checksum algorithm: {:?}", a)
            }
            WriterError::UnsupportedResourceType(t) => {
                write!(f, "Unsupported resource type: {:?}", t)
            }
            WriterError::UnknownResourceType(s) => write!(f, "Unknown resource type: {}", s),
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

fn extension_to_resource_type(ext: &str) -> Result<(ResourceType, PreProcessor<'_>), WriterError> {
    Ok(match ext {
        // Shader types
        "glsl" => (ResourceType::ShaderGLSL, compile_glsl_shader),
        "spv" => (ResourceType::ShaderSPIRV, dummy_preprocessor),
        "hlsl" => (ResourceType::ShaderHLSL, dummy_preprocessor),

        // Audio types
        "flac" => (ResourceType::AudioFLAC, resample_flac_file),
        "wav" => (ResourceType::AudioWAV, resample_wav_file),
        "ogg" => (ResourceType::AudioOGG, resample_ogg_file),

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

        _ => {
            // If the extension is not recognized, return an error
            return Err(WriterError::UnknownResourceType(ext.to_string()));
        }
    })
}

fn checksum<P: AsRef<std::path::Path>>(
    path: P,
    algorithm: ChecksumAlgorithm,
) -> Result<ResourceChecksum, WriterError> {
    let mut file = BufReader::new(File::open(path).map_err(WriterError::IoError)?);
    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .map_err(WriterError::IoError)?;

    let hash = match algorithm {
        ChecksumAlgorithm::Md5 => {
            let mut hasher = md5::Context::new();
            hasher.consume(&content);
            hasher.finalize().0
        }
        _ => {
            return Err(WriterError::UnsupportedChecksumAlgorithm(algorithm));
        }
    };

    Ok(ResourceChecksum::from_bytes(&hash))
}

fn create_manifest(create_options: WriteOptions, containers: &[Container]) -> Manifest {
    let mut headers = Vec::new();
    for container in containers {
        headers.push(container.metadata.header.clone());
    }

    Manifest {
        tool_created: "Yage2 Packager".to_string(),
        tool_version: "0.1.0".to_string(), // TODO: Get from Cargo.toml
        date_created: format_now().unwrap(),
        write_options: create_options,
        headers,
    }
}

fn validate_metadata<P: AsRef<std::path::Path>>(
    metadata_path: P,
    resource_type: ResourceType,
    name: &str,
) -> Result<ResourceMetadata, WriterError> {
    let metadata_content =
        std::fs::read_to_string(metadata_path.as_ref()).map_err(WriterError::IoError)?;
    let mut metadata: ResourceMetadata = toml::from_str(&metadata_content)
        .map_err(|e| WriterError::ParseMetadataFailed(name.to_string(), e))?;

    // If some fields are missing, the serde will fill them with defaults, 
    // But we need to ensure that the resource type matches
    if metadata.header.resource_type == ResourceType::Unknown {
        metadata.header.resource_type = resource_type;
    } else if metadata.header.resource_type != resource_type {
        return Err(WriterError::ValidateMetadataFailed(
            name.to_string(),
            format!(
                "Resource type mismatch: expected {:?}, found {:?}",
                resource_type, metadata.header.resource_type
            ),
        ));
    }

    // Ensure the type-specific metadata is set
    match metadata.type_specific {
        TypeSpecificMetadata::Unknown => {
            metadata.type_specific = TypeSpecificMetadata::default_for(resource_type);
        }
        _ => {
            // Ensure the type-specific metadata is suitable for the resource type
            if !metadata.type_specific.suitable_for(resource_type) {
                return Err(WriterError::ValidateMetadataFailed(
                    name.to_string(),
                    format!(
                        "Type-specific metadata {:?} is not suitable for resource type {:?}",
                        metadata.type_specific, resource_type
                    ),
                ));
            }
        }
    };

    // If the name is empty, use the provided name
    if metadata.header.name.is_empty() {
        metadata.header.name = name.to_string();
    }

    Ok(metadata)
}

fn create_metadata<P: AsRef<std::path::Path>>(
    resource_type: ResourceType,
    name: &str,
) -> Result<ResourceMetadata, WriterError> {
    let mut header = ResourceHeader::default();
    header.name = name.to_string();
    header.resource_type = resource_type;

    Ok(ResourceMetadata {
        header,
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
    options: &WriteOptions,
    directory: &TempDir,
) -> Result<Vec<std::path::PathBuf>, WriterError> {
    let mut containers = Vec::new();
    for file in input_files {
        let (we, ext) = split_path(&file);

        if ext == "toml" {
            // Skip metadata files
            debug!("Skipping metadata file: {}", we);
            continue;
        }

        let mut name = normalize_name(&file);
        let (resource_type, preprocessor) = extension_to_resource_type(&ext)?;
        info!("Processing file: {} (type: {:?})", we, resource_type);

        let toml = file.with_extension("toml");
        let mut metadata = if toml.exists() {
            // Metadata object found, validate it
            validate_metadata(&toml, resource_type, &name)?
        } else {
            // If file doesn't exist. Create a new one
            create_metadata::<P>(resource_type, &name)?
        };

        // Calculate the checksum, in case the preprocessor wants to use it
        metadata.header.checksum = checksum(&file, options.checksum_algorithm)?;
        // Preprocessor is responsible for modifying the file and metadata
        // and copying the file to the temp directory
        let dest_path = directory.path().join(&name);

        // TODO: What if the preprocessor changes the name?
        metadata =
            preprocessor(file, &metadata, &dest_path).map_err(WriterError::PreprocessorFailed)?;

        // If a user / preprocessor changed the name, we need to update it
        name = metadata.header.name.clone();
        // Update the metadata with the checksum
        metadata.header.checksum = checksum(&dest_path, options.checksum_algorithm)?;
        // Write the metadata to a temporary file
        let metadata_path = directory.path().join(&name).with_extension("toml");
        let metadata_content =
            toml::to_string(&metadata).map_err(WriterError::FormatMetadataFailed)?;
        std::fs::write(&metadata_path, metadata_content).map_err(WriterError::IoError)?;

        // Create the resource entry
        containers.push(Container {
            binary_path: dest_path,
            metadata_path,
            metadata,
        });
    }

    // Create the manifest
    let manifest = create_manifest(options.clone(), &containers);
    // Write the manifest to a temporary file
    let manifest_path = directory.path().join(".manifest.toml");
    let manifest_content = toml::to_string(&manifest).map_err(WriterError::FormatMetadataFailed)?;
    std::fs::write(&manifest_path, manifest_content).map_err(WriterError::IoError)?;

    // Make sure that manifest is the first file in the archive
    let mut files_to_archive = vec![manifest_path];
    for container in containers {
        files_to_archive.push(container.binary_path);
        files_to_archive.push(container.metadata_path);
    }

    info!("Created manifest with {} resources", files_to_archive.len());
    Ok(files_to_archive)
}

fn add_files<W>(
    tar_builder: &mut tar::Builder<W>,
    files: &[std::path::PathBuf],
) -> Result<(), WriterError>
where
    W: std::io::Write,
{
    for resource in files {
        let mut file = File::open(resource).map_err(WriterError::IoError)?;
        tar_builder
            .append_file(resource.file_name().unwrap(), &mut file)
            .map_err(WriterError::IoError)?;
    }

    Ok(())
}

/// Implementation of creating a YARC from a directory
/// This will involve reading files, normalizing names, and writing to a 
/// .tar or .tar.gz archive with the specified compression and checksum algorithm.
/// Optionally, for some file types, a preprocessor can be applied (e.g., resampling audio files).
pub fn write_from_directory<P: AsRef<std::path::Path>>(
    input_dir: P,
    options: WriteOptions,
    output: P,
) -> Result<(), WriterError> {
    // Read the directory and collect files based on the read mode
    let files =
        collect_files(input_dir, options.read_mode).map_err(WriterError::CollectingFilesFailed)?;

    // Group files with their metadata
    let temp_dir = TempDir::new("yage2_yarc").map_err(WriterError::IoError)?;
    let output_files = prepare_files::<P>(&files, &options.clone(), &temp_dir)?;

    // Create the output archive
    let output_path = output.as_ref();
    let output_file = std::fs::File::create(output_path).map_err(WriterError::TarError)?;
    match options.compression {
        Compression::None => {
            // Create a tar archive
            let mut tar = tar::Builder::new(output_file);
            add_files(&mut tar, &output_files)?;
            tar.finish().map_err(WriterError::TarError)?;
        }
        Compression::Gzip => {
            // Create a gzipped tar archive
            let enc = flate2::write::GzEncoder::new(output_file, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);
            add_files(&mut tar, &output_files)?;
            tar.finish().map_err(WriterError::TarError)?;
        }
    }

    Ok(())
}
