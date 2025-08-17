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
    pub length: usize, // In samples
}

impl Default for AudioAssetRaw {
    fn default() -> Self {
        AudioAssetRaw {
            data: vec![],
            sample_rate: 44100,
            channels: 2,
            length: 0,
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
    R32F,
    R64,
    RG8,
    RG16,
    RG32F,
    // TODO: Compressed formats
}

impl Default for PixelFormat {
    fn default() -> Self {
        PixelFormat::RGB(PixelDataType::U8)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

impl Default for TextureFilter {
    fn default() -> Self {
        TextureFilter::Nearest
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextureWrap {
    ClampToEdge,
    ClampToBorder,
    Repeat,
    MirroredRepeat,
}

impl Default for TextureWrap {
    fn default() -> Self {
        TextureWrap::ClampToEdge
    }
}

/// Internal representation of texture data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureAssetRaw {
    // Texture data is stored as an interleaved byte array,
    // in GPU-friendly format
    pub data: Vec<u8>,
    pub texture_type: TextureType,
    pub pixel_format: PixelFormat,
    pub use_mipmaps: bool,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    pub wrap_s: TextureWrap,
    pub wrap_t: TextureWrap,
    pub wrap_r: TextureWrap,
}

impl Default for TextureAssetRaw {
    fn default() -> Self {
        TextureAssetRaw {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MIDIEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    Idle { ms: f32 },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MIDIAssetRaw {
    pub events: Vec<MIDIEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssetRaw {
    Unknown,
    Shader(ShaderAssetRaw),
    Audio(AudioAssetRaw),
    Texture(TextureAssetRaw),
    MIDI(MIDIAssetRaw),
}

impl Default for AssetRaw {
    fn default() -> Self {
        AssetRaw::Unknown
    }
}
