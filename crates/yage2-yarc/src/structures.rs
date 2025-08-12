use serde::{Deserialize, Serialize};
use std::time::Instant;
use yage2_core::assets::reader::AssetHeader;
use yage2_core::assets::AssetType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShaderType {
    Fragment,
    Geometry,
    Vertex,
    Compute,
    TessellationControl,
}

impl Default for ShaderType {
    fn default() -> Self {
        ShaderType::Fragment
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShaderMetadata {
    #[serde(default)]
    pub shader_type: ShaderType,
    #[serde(default)]
    compile_options: Vec<String>,
}

impl Default for ShaderMetadata {
    fn default() -> Self {
        ShaderMetadata {
            shader_type: ShaderType::default(),
            compile_options: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioMetadata {}

impl Default for AudioMetadata {
    fn default() -> Self {
        AudioMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageMetadata {}

impl Default for ImageMetadata {
    fn default() -> Self {
        ImageMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FontMetadata {}

impl Default for FontMetadata {
    fn default() -> Self {
        FontMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelMetadata {}

impl Default for ModelMetadata {
    fn default() -> Self {
        ModelMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TypeSpecificMetadata {
    Unknown,
    Shader(ShaderMetadata),
    Audio(AudioMetadata),
    Image(ImageMetadata),
    Font(FontMetadata),
    Model(ModelMetadata),
}

impl TypeSpecificMetadata {
    pub fn default_for(asset_type: AssetType) -> Self {
        match asset_type {
            AssetType::ShaderGLSL | AssetType::ShaderSPIRV | AssetType::ShaderHLSL => {
                TypeSpecificMetadata::Shader(ShaderMetadata::default())
            }
            AssetType::AudioFLAC | AssetType::AudioWAV | AssetType::AudioOGG => {
                TypeSpecificMetadata::Audio(AudioMetadata::default())
            }
            AssetType::ImagePNG | AssetType::ImageJPEG | AssetType::ImageBMP => {
                TypeSpecificMetadata::Image(ImageMetadata::default())
            }
            AssetType::FontTTF | AssetType::FontOTF => {
                TypeSpecificMetadata::Font(FontMetadata::default())
            }
            AssetType::ModelOBJ | AssetType::ModelFBX | AssetType::ModelGLTF => {
                TypeSpecificMetadata::Model(ModelMetadata::default())
            }
            _ => TypeSpecificMetadata::Unknown,
        }
    }

    pub fn suitable_for(&self, asset_type: AssetType) -> bool {
        match self {
            TypeSpecificMetadata::Shader(_) => {
                asset_type == AssetType::ShaderGLSL
                    || asset_type == AssetType::ShaderSPIRV
                    || asset_type == AssetType::ShaderHLSL
            }
            TypeSpecificMetadata::Audio(_) => {
                matches!(
                    asset_type,
                    AssetType::AudioFLAC | AssetType::AudioWAV | AssetType::AudioOGG
                )
            }
            TypeSpecificMetadata::Image(_) => {
                matches!(
                    asset_type,
                    AssetType::ImagePNG | AssetType::ImageJPEG | AssetType::ImageBMP
                )
            }
            TypeSpecificMetadata::Font(_) => {
                matches!(asset_type, AssetType::FontTTF | AssetType::FontOTF)
            }
            TypeSpecificMetadata::Model(_) => {
                matches!(
                    asset_type,
                    AssetType::ModelOBJ | AssetType::ModelFBX | AssetType::ModelGLTF
                )
            }
            TypeSpecificMetadata::Unknown => false,
        }
    }
}

impl Default for TypeSpecificMetadata {
    fn default() -> Self {
        TypeSpecificMetadata::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AssetMetadata {
    #[serde(default)]
    pub header: AssetHeader,
    #[serde(default)]
    pub type_specific: TypeSpecificMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Compression {
    None,
    Gzip,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ReadMode {
    Flat,
    Recursive,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ChecksumAlgorithm {
    Md5,
    Blake3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct WriteOptions {
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub checksum_algorithm: ChecksumAlgorithm,
}

fn serialize_instant<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let duration = instant.elapsed();
    serializer.serialize_u64(duration.as_secs() * 1_000 + u64::from(duration.subsec_millis()))
}

fn deserialize_instant<'de, D>(deserializer: D) -> Result<Instant, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let timestamp = u64::deserialize(deserializer)?;
    Ok(Instant::now() - std::time::Duration::from_millis(timestamp))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub tool_created: String,
    pub tool_version: String,
    #[serde(
        serialize_with = "serialize_instant",
        deserialize_with = "deserialize_instant"
    )]
    pub date_created: Instant,
    pub write_options: WriteOptions,
    pub headers: Vec<AssetHeader>,
}
