use serde::{Deserialize, Serialize};
use yage2_core::resources::{ResourceHeader, ResourceType};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum ShaderType {
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
pub(crate) struct ShaderMetadata {
    #[serde(default)]
    shader_type: ShaderType,
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
pub(crate) struct AudioMetadata {}

impl Default for AudioMetadata {
    fn default() -> Self {
        AudioMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ImageMetadata {}

impl Default for ImageMetadata {
    fn default() -> Self {
        ImageMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct FontMetadata {}

impl Default for FontMetadata {
    fn default() -> Self {
        FontMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ModelMetadata {}

impl Default for ModelMetadata {
    fn default() -> Self {
        ModelMetadata {}
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) enum TypeSpecificMetadata {
    Unknown,
    Shader(ShaderMetadata),
    Audio(AudioMetadata),
    Image(ImageMetadata),
    Font(FontMetadata),
    Model(ModelMetadata),
}

impl TypeSpecificMetadata {
    pub fn default_for(resource_type: ResourceType) -> Self {
        match resource_type {
            ResourceType::ShaderGLSL | ResourceType::ShaderSPIRV | ResourceType::ShaderHLSL => {
                TypeSpecificMetadata::Shader(ShaderMetadata::default())
            }
            ResourceType::AudioFLAC
            | ResourceType::AudioWAV
            | ResourceType::AudioOGG
            | ResourceType::AudioMP3 => TypeSpecificMetadata::Audio(AudioMetadata::default()),
            ResourceType::ImagePNG | ResourceType::ImageJPEG | ResourceType::ImageBMP => {
                TypeSpecificMetadata::Image(ImageMetadata::default())
            }
            ResourceType::FontTTF | ResourceType::FontOTF => {
                TypeSpecificMetadata::Font(FontMetadata::default())
            }
            ResourceType::ModelOBJ | ResourceType::ModelFBX | ResourceType::ModelGLTF => {
                TypeSpecificMetadata::Model(ModelMetadata::default())
            }
            _ => TypeSpecificMetadata::Unknown,
        }
    }

    pub fn suitable_for(&self, resource_type: ResourceType) -> bool {
        match self {
            TypeSpecificMetadata::Shader(_) => {
                resource_type == ResourceType::ShaderGLSL
                    || resource_type == ResourceType::ShaderSPIRV
                    || resource_type == ResourceType::ShaderHLSL
            }
            TypeSpecificMetadata::Audio(_) => {
                matches!(
                    resource_type,
                    ResourceType::AudioFLAC
                        | ResourceType::AudioWAV
                        | ResourceType::AudioOGG
                        | ResourceType::AudioMP3
                )
            }
            TypeSpecificMetadata::Image(_) => {
                matches!(
                    resource_type,
                    ResourceType::ImagePNG | ResourceType::ImageJPEG | ResourceType::ImageBMP
                )
            }
            TypeSpecificMetadata::Font(_) => {
                matches!(resource_type, ResourceType::FontTTF | ResourceType::FontOTF)
            }
            TypeSpecificMetadata::Model(_) => {
                matches!(
                    resource_type,
                    ResourceType::ModelOBJ | ResourceType::ModelFBX | ResourceType::ModelGLTF
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct ResourceMetadata {
    #[serde(default)]
    pub(crate) common: ResourceHeader,
    #[serde(default)]
    pub(crate) type_specific: TypeSpecificMetadata,
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
pub enum HashAlgorithm {
    Md5,
    Blake3,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct YARCWriteOptions {
    pub compression: Compression,
    pub read_mode: ReadMode,
    pub hash_algorithm: HashAlgorithm,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct YARCManifest {
    pub tool_created: String,
    pub tool_version: String,
    pub date_created: String,
    pub write_options: YARCWriteOptions,
    pub resources: Vec<ResourceHeader>,
}
