use crate::ir::mesh::{
    IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRMeshVertex, IRTopology,
};
use crate::AssetID;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::mem::offset_of;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
#[repr(packed)]
pub struct IRGlyphVertex {
    pub position: [f32; 2],
    // The Tex coord can be calculated from the
    // position by dividing by the atlas size.
    // So omitting it here to save space.
}

impl IRGlyphVertex {
    pub fn new(pos: Vec2) -> Self {
        Self {
            position: pos.to_array(),
        }
    }

    pub fn layout() -> [IRLayout; 1] {
        [IRLayout {
            field: IRLayoutField::Position,
            sample_type: IRLayoutSampleType::Float,
            samples: 2, // floats
            stride_bytes: size_of::<IRGlyphVertex>(),
            offset_bytes: offset_of!(IRGlyphVertex, position),
        }]
    }

    pub fn into_bytes<'a>(self) -> &'a [u8] {
        unsafe {
            std::slice::from_raw_parts(
                (&self as *const IRGlyphVertex) as *const u8,
                size_of::<IRGlyphVertex>(),
            )
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRGlyph {
    pub vertex_offset: usize,
    pub vertex_count: usize,

    pub x_advance: f32,
    pub y_offset: f32,
    pub x_offset: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IRFont {
    pub glyphs: HashMap<char, IRGlyph>,
    pub y_advance: f32,
    pub atlas: AssetID,

    #[serde(with = "serde_bytes")]
    pub vertices: Vec<u8>,
    pub topology: IRTopology,
    // Since for each glyph we only need to draw two triangles.
    // Index array can save 1 vertex per glyph that is 2 floats - 8 bytes.
    // Having 6 vertices per glyph is 24 bytes (when using 32 bit indices)
    // and 12 bytes (when using 16 bit indices).
    // Having the indices is not beneficial.
}

impl IRFont {
    pub fn memory_usage(&self) -> usize {
        let mut sum = 0;
        // TODO: calculate memory usage
        sum
    }
}
