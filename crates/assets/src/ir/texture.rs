use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRTextureType {
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

impl Default for IRTextureType {
    fn default() -> Self {
        IRTextureType::Unknown
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IRPixelDataType {
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
pub enum IRPixelFormat {
    Unknown,
    /// Red only.
    R8,
    /// Red, green.
    R8G8,
    /// Red, green, blue.
    R8G8B8,
    /// Red, green, blue, alpha.
    R8G8B8A8,
    /// Red only (16 bits).
    R16,
    /// Red, green (16 bits).
    R16G16,
    /// Red, green, blue (16 bits).
    R16G16B16,
    /// Red, green, blue, alpha (16 bits).
    R16G16B16A16,
    /// Red, green, blue (32 bits float)
    R32G32B32FLOAT,
    /// Red, green, blue, alpha (32 bits float)
    R32G32B32A32FLOAT,
}

impl Default for IRPixelFormat {
    fn default() -> Self {
        IRPixelFormat::R8G8B8
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRTextureFilter {
    Nearest,
    Linear,
}

impl Default for IRTextureFilter {
    fn default() -> Self {
        IRTextureFilter::Linear
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRTextureWrap {
    ClampToEdge,
    ClampToBorder,
    Repeat,
    MirroredRepeat,
}

impl Default for IRTextureWrap {
    fn default() -> Self {
        IRTextureWrap::Repeat
    }
}

/// Internal representation of texture data
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct IRTexture {
    // Texture data is stored as an interleaved byte array,
    // in GPU-friendly format
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub texture_type: IRTextureType,
    pub pixel_format: IRPixelFormat,
    pub use_mipmaps: bool,
    pub min_filter: IRTextureFilter,
    pub mag_filter: IRTextureFilter,
    pub wrap_s: IRTextureWrap,
    pub wrap_t: IRTextureWrap,
    pub wrap_r: IRTextureWrap,
}

impl Debug for IRTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRTexture")
            .field("data_length", &self.data.len())
            .field("texture_type", &self.texture_type)
            .field("pixel_format", &self.pixel_format)
            .field("use_mipmaps", &self.use_mipmaps)
            .field("min_filter", &self.min_filter)
            .field("mag_filter", &self.mag_filter)
            .field("wrap_s", &self.wrap_s)
            .field("wrap_t", &self.wrap_t)
            .field("wrap_r", &self.wrap_r)
            .finish()
    }
}

impl Default for IRTexture {
    fn default() -> Self {
        IRTexture {
            data: vec![],
            texture_type: Default::default(),
            pixel_format: Default::default(),
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        }
    }
}

impl IRTexture {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRTexture>();
        sum += self.data.capacity();
        sum
    }
}
