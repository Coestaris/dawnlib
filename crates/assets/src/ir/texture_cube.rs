use crate::ir::texture2d::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRTextureCubeOrder {
    OpenGL, // +X, -X, +Y, -Y, +Z, -Z
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRTextureCubeSide {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl IRTextureCubeOrder {
    pub fn to_sides(&self) -> [IRTextureCubeSide; 6] {
        match self {
            IRTextureCubeOrder::OpenGL => [
                IRTextureCubeSide::PositiveX,
                IRTextureCubeSide::NegativeX,
                IRTextureCubeSide::PositiveY,
                IRTextureCubeSide::NegativeY,
                IRTextureCubeSide::PositiveZ,
                IRTextureCubeSide::NegativeZ,
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct IRTextureCubeSideData {
    // Texture data is stored as an interleaved byte array,
    // in GPU-friendly format
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct IRTextureCube {
    pub sides: [IRTextureCubeSideData; 6],
    pub order: IRTextureCubeOrder,
    pub size: u32,

    pub pixel_format: IRPixelFormat,
    pub use_mipmaps: bool,
    pub min_filter: IRTextureFilter,
    pub mag_filter: IRTextureFilter,
    pub wrap_s: IRTextureWrap,
    pub wrap_t: IRTextureWrap,
    pub wrap_r: IRTextureWrap,
}

impl Debug for IRTextureCube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRTextureCube")
            .field("data_length", &self.sides.len())
            .field("order", &self.order)
            .field("size", &self.size)
            .field("pixel_format", &self.pixel_format)
            .field("use_mipmaps", &self.use_mipmaps)
            .field("min_filter", &self.min_filter)
            .field("mag_filter", &self.mag_filter)
            .field("wrap_s", &self.wrap_s)
            .finish()
    }
}

impl Default for IRTextureCube {
    fn default() -> Self {
        IRTextureCube {
            sides: [
                IRTextureCubeSideData { data: vec![] },
                IRTextureCubeSideData { data: vec![] },
                IRTextureCubeSideData { data: vec![] },
                IRTextureCubeSideData { data: vec![] },
                IRTextureCubeSideData { data: vec![] },
                IRTextureCubeSideData { data: vec![] },
            ],
            order: IRTextureCubeOrder::OpenGL,
            size: 0,
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

impl IRTextureCube {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRTextureCube>();
        for side in &self.sides {
            sum += side.data.len();
        }
        sum
    }
}
