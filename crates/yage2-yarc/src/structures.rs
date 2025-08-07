use serde::{Deserialize, Serialize};
use yage2_core::resources::reader::ResourceHeader;
use yage2_core::resources::resource::ResourceType;

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
    pub fn default_for(resource_type: ResourceType) -> Self {
        match resource_type {
            ResourceType::ShaderGLSL | ResourceType::ShaderSPIRV | ResourceType::ShaderHLSL => {
                TypeSpecificMetadata::Shader(ShaderMetadata::default())
            }
            ResourceType::AudioFLAC | ResourceType::AudioWAV | ResourceType::AudioOGG => {
                TypeSpecificMetadata::Audio(AudioMetadata::default())
            }
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
                    ResourceType::AudioFLAC | ResourceType::AudioWAV | ResourceType::AudioOGG
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ResourceMetadata {
    #[serde(default)]
    pub header: ResourceHeader,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub tool_created: String,
    pub tool_version: String,
    pub date_created: String,
    pub write_options: WriteOptions,
    pub headers: Vec<ResourceHeader>,
}
