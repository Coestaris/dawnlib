use crate::assets::AssetType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetChecksum([u8; 16]);

impl AssetChecksum {
    pub fn from_bytes(bytes: &[u8]) -> AssetChecksum {
        let mut checksum = [0; 16];
        let len = bytes.len().min(16);
        checksum[..len].copy_from_slice(&bytes[..len]);
        AssetChecksum(checksum)
    }
}

impl Default for AssetChecksum {
    fn default() -> Self {
        AssetChecksum([0; 16])
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetHeader {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub asset_type: AssetType,
    #[serde(default)]
    pub checksum: AssetChecksum,
}

impl Default for AssetHeader {
    fn default() -> Self {
        AssetHeader {
            name: String::new(),
            tags: Vec::new(),
            asset_type: AssetType::Unknown,
            checksum: AssetChecksum::default(),
        }
    }
}

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
