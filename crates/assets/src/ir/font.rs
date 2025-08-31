use crate::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use crate::AssetID;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::mem::offset_of;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
#[repr(packed)]
pub struct IRGlyphVertex {
    /// Zero based position in the bounding quad for the glyph.
    /// This is used to calculate the final position in the vertex shader.
    pub position: [f32; 2],
    /// Texture coordinate in the font atlas.
    pub tex_coord: [f32; 2],
}

impl IRGlyphVertex {
    pub fn new(pos: Vec2, tex: Vec2) -> Self {
        Self {
            position: pos.to_array(),
            tex_coord: tex.to_array(),
        }
    }

    pub fn layout() -> [IRLayout; 2] {
        [
            IRLayout {
                field: IRLayoutField::Position,
                sample_type: IRLayoutSampleType::Float,
                samples: 2, // floats
                stride_bytes: size_of::<IRGlyphVertex>(),
                offset_bytes: offset_of!(IRGlyphVertex, position),
            },
            IRLayout {
                field: IRLayoutField::TexCoord,
                sample_type: IRLayoutSampleType::Float,
                samples: 2, // floats
                stride_bytes: size_of::<IRGlyphVertex>(),
                offset_bytes: offset_of!(IRGlyphVertex, tex_coord),
            },
        ]
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
    pub index_offset: usize,
    pub index_count: usize,
    pub x_advance: f32,
    pub y_offset: f32,
    pub x_offset: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IRFont {
    pub glyphs: HashMap<char, IRGlyph>,
    pub y_advance: f32,
    pub space_advance: f32,
    pub atlas: AssetID,

    #[serde(with = "serde_bytes")]
    pub vertices: Vec<u8>,
    pub topology: IRTopology,

    // For each glyph we only need to draw two triangles.
    // Index array can save 2 vertex per glyph that is 2*4 floats - 32 bytes.
    // Having 6 indices per glyph adds 12 bytes (when using 16 bit indices)
    // So having index buffer saves 20 bytes per glyph.
    #[serde(with = "serde_bytes")]
    pub indices: Vec<u8>,
    pub index_type: IRIndexType,
}

impl IRFont {
    pub fn memory_usage(&self) -> usize {
        let sum = 0;
        // TODO: calculate memory usage
        sum
    }
}

impl Debug for IRFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRFont")
            .field("glyphs", &self.glyphs.len())
            .field("y_advance", &self.y_advance)
            .field("atlas", &self.atlas)
            .field("vertices", &self.vertices.len())
            .field("topology", &self.topology)
            .field("indices", &self.indices.len())
            .field("index_type", &self.index_type)
            .finish()
    }
}
