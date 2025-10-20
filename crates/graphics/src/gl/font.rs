use crate::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use crate::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use crate::gl::raii::vertex_array::VertexArray;
use crate::passes::events::PassEventTrait;
use crate::passes::result::RenderResult;
use dawn_assets::ir::font::{IRFont, IRGlyph, IRGlyphVertex};
use dawn_assets::{Asset, AssetCastable, AssetID, AssetMemoryUsage};
use glam::Vec2;
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
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
    vao: VertexArray,
    _vbo: ArrayBuffer,

    pub glyphs: HashMap<char, IRGlyph>,
    pub atlas: Asset,
    pub y_advance: f32,
    pub space_advance: f32,
}

impl AssetCastable for Font {}

impl Font {
    pub(crate) fn from_ir<E: PassEventTrait>(
        gl: Arc<glow::Context>,
        ir: IRFont,
        deps: HashMap<AssetID, Asset>,
    ) -> Result<(Self, AssetMemoryUsage), FontError> {
        debug!("Creating Font from IR: {ir:?}");

        let vao = VertexArray::new(gl.clone(), ir.topology, ir.index_type)
            .ok_or(FontError::VertexArrayAllocationFailed)?;
        let mut vbo = ArrayBuffer::new(gl.clone()).ok_or(FontError::ArrayBufferAllocationFailed)?;
        let mut ebo = ElementArrayBuffer::new(gl.clone())
            .ok_or(FontError::ElementArrayBufferAllocationFailed)?;

        VertexArray::bind(&gl, &vao);
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        vbo_binding.feed(&ir.vertices, ArrayBufferUsage::StaticDraw);
        ebo_binding.feed(&ir.indices, ElementArrayBufferUsage::StaticDraw);

        for (i, layout) in IRGlyphVertex::layout().iter().enumerate() {
            vao.setup_attribute(i as u32, layout);
        }

        drop(vbo_binding);
        VertexArray::unbind(&gl);

        let atlas = deps
            .get(&ir.atlas)
            .cloned()
            .ok_or_else(|| FontError::AtlasNoFound(ir.atlas.clone()))?;

        Ok((
            Font {
                glyphs: ir.glyphs,
                atlas,
                y_advance: ir.y_advance,
                space_advance: ir.space_advance,
                vao,
                _vbo: vbo,
            },
            AssetMemoryUsage::new(0, 0),
        ))
    }

    pub fn render_string(
        &self,
        gl: &glow::Context,
        string: &str,
        mut on_glyph: impl FnMut(char, Option<&IRGlyph>) -> (bool, RenderResult),
    ) -> RenderResult {
        let mut result = RenderResult::default();

        VertexArray::bind(gl, &self.vao);
        for c in string.chars() {
            let glyph = self.glyphs.get(&c).map_or_else(|| None, |g| Some(g));

            let (skip, new_result) = on_glyph(c, glyph);
            result += new_result;

            if skip {
                continue;
            }

            let glyph = glyph.unwrap_or_else(|| {
                panic!("Glyph for character '{}' not found in font", c);
            });
            result += self
                .vao
                .draw_elements(glyph.index_count, glyph.index_offset);
        }
        VertexArray::unbind(gl);

        result
    }

    pub fn text_dimensions(&self, string: &str) -> Vec2 {
        let mut width = 0.0;
        let mut height = self.y_advance;

        for c in string.chars() {
            match c {
                ' ' => {
                    width += self.space_advance; // Simple space handling
                }
                '\n' => {
                    width = 0.0;
                    height += self.y_advance;
                }
                _ => {
                    if let Some(glyph) = self.glyphs.get(&c) {
                        width += glyph.x_advance;
                    }
                }
            }
        }

        Vec2::new(width, height)
    }
}
