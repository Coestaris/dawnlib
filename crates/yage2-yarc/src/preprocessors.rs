use crate::structures::{AssetMetadata, ShaderType, TypeSpecificMetadata};
use log::{debug, info};

#[derive(Debug)]
pub enum PreprocessorsError {
    GlslCompilerNotFound,
    FFMpegNotFound,
    InvalidMetadata(String),
    IOError(std::io::Error),
    CompilationFailed(String),
    ConversionFailed(String),
}

impl std::fmt::Display for PreprocessorsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessorsError::GlslCompilerNotFound => write!(f, "GLSL compiler not found"),
            PreprocessorsError::FFMpegNotFound => write!(f, "FFMpeg not found"),
            PreprocessorsError::InvalidMetadata(msg) => write!(f, "Invalid metadata: {}", msg),
            PreprocessorsError::IOError(e) => write!(f, "I/O error: {}", e),
            PreprocessorsError::CompilationFailed(msg) => {
                write!(f, "GLSL Compilation failed: {}", msg)
            }
            PreprocessorsError::ConversionFailed(msg) => {
                write!(f, "FFMpeg Conversion failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for PreprocessorsError {}

/// Preprocessor function type that takes a file path and metadata,
/// and returns a processed file path or an error.
pub type PreProcessor<'a> = fn(
    &'a std::path::PathBuf,
    &AssetMetadata,
    &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError>;

fn get_glslc_path() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "windows") {
        Some(std::path::PathBuf::from("glslc"))
    } else if cfg!(target_os = "linux") {
        Some(std::path::PathBuf::from("glslc"))
    } else if cfg!(target_os = "macos") {
        // Assuming glslc is installed via Homebrew or similar
        // TODO: Allow user to configure glslc path
        Some(std::path::PathBuf::from("glslang"))
    } else {
        None
    }
}

fn get_ffmpeg_path() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from("ffmpeg"))
}

fn get_cache_directory() -> std::path::PathBuf {
    // TODO: Allow user to configure cache directory
    dirs::data_local_dir().unwrap().join("yage2").join("cache")
}

fn entry_name(name: &str, metadata: &AssetMetadata) -> String {
    let hash_hex = format!("{:x}", md5::compute(format!("{}{:?}", name, metadata)));
    format!("{}.cache", hash_hex)
}

fn is_cache_exists(name: &str, metadata: &AssetMetadata) -> Option<std::path::PathBuf> {
    let cache_dir = get_cache_directory();
    let cache_path = cache_dir.join(entry_name(name, metadata));

    if cache_path.exists() {
        Some(cache_path)
    } else {
        None
    }
}

fn create_cache_entry(
    name: &str,
    metadata: &AssetMetadata,
    file: &std::path::PathBuf,
) -> Result<std::path::PathBuf, PreprocessorsError> {
    let cache_dir = get_cache_directory();
    std::fs::create_dir_all(&cache_dir).map_err(|e| PreprocessorsError::IOError(e))?;

    debug!("Creating cache entry for: {}", name);
    let cache_path = cache_dir.join(entry_name(name, metadata));
    if !cache_path.exists() {
        std::fs::copy(file, &cache_path).map_err(|e| PreprocessorsError::IOError(e))?;
    }
    Ok(cache_path)
}

/// In the cache directory, create the file with the name of the hash of the
/// file name and metadata. If the file already exists, return the path to the file.
/// If the file does not exist, create it and return the path to the file.
macro_rules! cache_me {
    ($metadata:expr, $output_path:expr, $new_value:expr) => {
        if let Some(cache_path) = is_cache_exists(&$metadata.header.name, &$metadata) {
            debug!("Cache hit for: {}", $metadata.header.name);
            std::fs::copy(cache_path, $output_path).map_err(|e| PreprocessorsError::IOError(e))?;
            Ok($metadata.clone())
        } else {
            debug!("Cache miss for: {}", $metadata.header.name);
            let val = $new_value?;
            create_cache_entry(&$metadata.header.name, $metadata, $output_path)?;
            Ok(val)
        }
    };
}

pub fn dummy_preprocessor<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    info!("Copying file: {}", metadata.header.name);

    std::fs::copy(path, output_path).map_err(|e| PreprocessorsError::IOError(e))?;

    Ok(metadata.clone())
}

fn compile_glsl_shader_impl<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    info!("Compiling GLSL shader: {}", metadata.header.name);

    let glslc_path = get_glslc_path().ok_or(PreprocessorsError::GlslCompilerNotFound)?;
    let mut command = std::process::Command::new(glslc_path);
    if let TypeSpecificMetadata::Shader(shader_metadata) = &metadata.type_specific {
        match shader_metadata.shader_type {
            ShaderType::Vertex => command.arg("-fshader-stage=vertex"),
            ShaderType::Fragment => command.arg("-fshader-stage=fragment"),
            ShaderType::Compute => command.arg("-fshader-stage=compute"),
            ShaderType::Geometry => command.arg("-fshader-stage=geometry"),
            ShaderType::TessellationControl => command.arg("-fshader-stage=tessellation_control"),
        };
    } else {
        return Err(PreprocessorsError::InvalidMetadata(
            "Expected shader metadata".to_string(),
        ));
    }
    command.arg(path);
    command
        .arg("--target-env=vulkan1.2") // Assuming Vulkan 1.2 as the target environment
        .arg("-o")
        .arg(output_path);

    // Print the command for debugging
    debug!("Running command: {:?}", command);
    let output = command
        .output()
        .map_err(|e| PreprocessorsError::IOError(e))?;
    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(PreprocessorsError::CompilationFailed(
            error_message.to_string(),
        ));
    }

    Ok(metadata.clone())
}

pub(crate) fn compile_glsl_shader<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    cache_me!(
        metadata,
        output_path,
        compile_glsl_shader_impl(path, metadata, output_path)
    )
}

const DESTIONATION_SAMPLE_RATE: u32 = 48_000;

pub fn resample_audio_file<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    format: &str,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    let ffmpeg_path = get_ffmpeg_path().ok_or(PreprocessorsError::FFMpegNotFound)?;
    let mut command = std::process::Command::new(ffmpeg_path);
    command.arg("-i").arg(path);
    command
        .arg("-ar")
        .arg(DESTIONATION_SAMPLE_RATE.to_string())
        .arg("-ac")
        .arg("2") // Assuming stereo output
        .arg("-f")
        .arg(format)
        .arg(output_path);

    // Print the command for debugging
    debug!("Running command: {:?}", command);

    let output = command
        .output()
        .map_err(|e| PreprocessorsError::IOError(e))?;
    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(PreprocessorsError::ConversionFailed(
            error_message.to_string(),
        ));
    }

    Ok(metadata.clone())
}

pub fn resample_ogg_file<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    info!("Resampling OGG file: {}", metadata.header.name);
    cache_me!(
        metadata,
        output_path,
        resample_audio_file(path, metadata, "ogg", output_path)
    )
}

pub fn resample_flac_file<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    info!("Resampling FLAC file: {}", metadata.header.name);
    cache_me!(
        metadata,
        output_path,
        resample_audio_file(path, metadata, "flac", output_path)
    )
}

pub fn resample_wav_file<'a>(
    path: &'a std::path::PathBuf,
    metadata: &AssetMetadata,
    output_path: &'a std::path::PathBuf,
) -> Result<AssetMetadata, PreprocessorsError> {
    info!("Resampling WAV file: {}", metadata.header.name);
    cache_me!(
        metadata,
        output_path,
        resample_audio_file(path, metadata, "wav", output_path)
    )
}
