use crate::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use crate::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use crate::gl::raii::vertex_array::VertexArray;
use crate::passes::events::PassEventTrait;
use crate::passes::result::RenderResult;
use dawn_assets::ir::font::{IRFont, IRGlyph, IRGlyphVertex};
use dawn_assets::ir::mesh::IRIndexType;
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage};
use log::debug;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FontError {
    #[error("Atlas with ID '{0}' not found for font")]
    AtlasNoFound(AssetID),
    #[error("Failed to allocate VertexArray")]
    VertexArrayAllocationFailed,
    #[error("Failed to allocate ArrayBuffer")]
    ArrayBufferAllocationFailed,
    #[error("Failed to allocate ElementArrayBuffer")]
    ElementArrayBufferAllocationFailed,
}

#[derive(Debug)]
pub struct Font {
    pub glyphs: HashMap<char, IRGlyph>,
    pub atlas: Asset,
    pub y_advance: f32,

    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
}

impl AssetCastable for Font {}

impl Font {
    pub(crate) fn from_ir<E: PassEventTrait>(
        ir: IRFont,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), FontError> {
        debug!("Creating Font from IR: {ir:?}");

        let vao = VertexArray::new(ir.topology, ir.index_type)
            .ok_or(FontError::VertexArrayAllocationFailed)?;
        let mut vbo = ArrayBuffer::new().ok_or(FontError::ArrayBufferAllocationFailed)?;
        let mut ebo =
            ElementArrayBuffer::new().ok_or(FontError::ElementArrayBufferAllocationFailed)?;

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        vbo_binding.feed(&ir.vertices, ArrayBufferUsage::StaticDraw);
        ebo_binding.feed(&ir.indices, ElementArrayBufferUsage::StaticDraw);

        for (i, layout) in IRGlyphVertex::layout().iter().enumerate() {
            vao_binding.setup_attribute(i, layout);
        }

        drop(vbo_binding);
        drop(vao_binding);

        let atlas = deps
            .get(&ir.atlas)
            .cloned()
            .ok_or_else(|| FontError::AtlasNoFound(ir.atlas.clone()))?;

        Ok((
            Font {
                glyphs: ir.glyphs,
                atlas,
                y_advance: ir.y_advance,
                vao,
                vbo,
            },
            AssetMemoryUsage::new(0, 0),
        ))
    }

    pub fn render_string(
        &self,
        string: &str,
        mut on_glyph: impl FnMut(&IRGlyph) -> (bool, RenderResult),
    ) -> RenderResult {
        let mut result = RenderResult::default();

        let biding = self.vao.bind();
        for c in string.chars() {
            let glyph = self.glyphs.get(&c).unwrap();
            let (skip, new_result) = on_glyph(glyph);
            result += new_result;

            if skip {
                continue;
            }

            result += biding.draw_elements(glyph.index_count, glyph.index_offset);
        }

        result
    }
}
