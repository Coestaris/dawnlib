use crate::assets::AssetType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderSourceType {
    Fragment,
    Geometry,
    Vertex,
    Compute,
    TessellationControl,

    /* Precompiled */
    PrecompiledFragment,
    PrecompiledGeometry,
    PrecompiledVertex,
    PrecompiledCompute,
    PrecompiledTessellationControl,
}

impl Default for ShaderSourceType {
    fn default() -> Self {
        ShaderSourceType::Fragment
    }
}

/// Internal representation of shader data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShaderAssetRaw {
    pub compile_options: Vec<String>,
    pub sources: HashMap<ShaderSourceType, Vec<u8>>,
}

impl Default for ShaderAssetRaw {
    fn default() -> Self {
        ShaderAssetRaw {
            compile_options: vec![],
            sources: Default::default(),
        }
    }
}

/// Internal representation of audio data
/// Always storing samples in the F32 sample format
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioAssetRaw {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u8,
    pub duration: usize, // In samples
}

impl Default for AudioAssetRaw {
    fn default() -> Self {
        AudioAssetRaw {
            data: vec![],
            sample_rate: 44100,
            channels: 2,
            duration: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureType {
    Unknown,
    Texture1D {
        width: u32,
    },
    Texture2D {
        width: u32,
        height: u32,
    },
    TextureCube {
        size: u32,
    },
    Texture3D {
        width: u32,
        height: u32,
        depth: u32,
    },
    Texture2DArray {
        width: u32,
        height: u32,
        layers: u32,
    },
    TextureCubeArray {
        size: u32,
        layers: u32,
    },
    Texture2DMultisample {
        width: u32,
        height: u32,
        samples: u32,
    },
    Texture2DMultisampleArray {
        width: u32,
        height: u32,
        layers: u32,
        samples: u32,
    },
    TextureBuffer {
        size: u32,
    },
}

impl Default for TextureType {
    fn default() -> Self {
        TextureType::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelDataType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Unknown,
    RGBA(PixelDataType),
    RGB(PixelDataType),
    BGRA(PixelDataType),
    BGR(PixelDataType),
    SRGB(PixelDataType),
    SRGBA(PixelDataType),
    R8,
    R16,
    R32,
    R64,
    RG8,
    RG16,
    RG32,
    RG64,
    // TODO: Compressed formats
}

impl Default for PixelFormat {
    fn default() -> Self {
        PixelFormat::Unknown
    }
}

/// Internal representation of texture data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssetRaw {
    #[serde(default)]
    pub data: Vec<u8>,
    #[serde(default)]
    pub texture_type: TextureType,
    #[serde(default)]
    pub pixel_format: PixelFormat,
}

impl Default for TextureAssetRaw {
    fn default() -> Self {
        TextureAssetRaw {
            data: vec![],
            texture_type: TextureType::Texture2D {
                width: 0,
                height: 0,
            },
            pixel_format: PixelFormat::RGBA(PixelDataType::U8),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssetRaw {
    Unknown,
    Shader(ShaderAssetRaw),
    Audio(AudioAssetRaw),
    Texture(TextureAssetRaw),
}

impl Default for AssetRaw {
    fn default() -> Self {
        AssetRaw::Unknown
    }
}
