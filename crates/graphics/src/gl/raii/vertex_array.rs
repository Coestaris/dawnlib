use crate::passes::result::RenderResult;
use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutSampleType, IRTopology};
use glow::HasContext;
use log::debug;
use std::sync::Arc;

#[derive(Debug)]
pub struct VertexArray {
    gl: Arc<glow::Context>,
    id: glow::VertexArray,
    draw_mode: u32,
    topology_size: usize,
    index_type: u32,
    index_size: usize,
}

pub struct VertexArrayBinding<'a> {
    gl: &'a glow::Context,
    vertex_array: &'a VertexArray,
}

impl<'a> VertexArrayBinding<'a> {
    #[inline(always)]
    fn new(gl: &'a glow::Context, vertex_array: &'a VertexArray) -> Self {
        unsafe {
            gl.bind_vertex_array(Some(vertex_array.as_inner()));
        }
        Self { gl, vertex_array }
    }

    pub fn setup_attribute(&self, index: u32, attribute: &IRLayout) {
        let gl_format = match attribute.sample_type {
            IRLayoutSampleType::Float => glow::FLOAT,
            IRLayoutSampleType::U32 => glow::UNSIGNED_INT,
        };

        unsafe {
            self.gl.enable_vertex_attrib_array(index);
            self.gl.vertex_attrib_pointer_f32(
                index,
                attribute.samples as i32,
                gl_format,
                false,
                attribute.stride_bytes as i32,
                attribute.offset_bytes as i32,
            );
        }
    }

    #[inline(always)]
    pub fn draw_elements_base_vertex(
        &self,
        index_count: usize,
        index_offset: usize,
        base_vertex: usize,
    ) -> RenderResult {
        unsafe {
            self.gl.draw_elements_base_vertex(
                self.vertex_array.draw_mode,
                index_count as i32,
                self.vertex_array.index_type,
                (index_offset * self.vertex_array.index_size) as i32,
                base_vertex as i32,
            );
        }

        RenderResult::ok(1, index_count / self.vertex_array.topology_size)
    }

    pub fn draw_elements(&self, index_count: usize, index_offset: usize) -> RenderResult {
        unsafe {
            self.gl.draw_elements(
                self.vertex_array.draw_mode,
                index_count as i32,
                self.vertex_array.index_type,
                (index_offset * self.vertex_array.index_size) as i32,
            );
        }

        RenderResult::ok(1, index_count / self.vertex_array.topology_size)
    }
}

impl<'a> Drop for VertexArrayBinding<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            self.gl.bind_vertex_array(None);
        }
    }
}

impl VertexArray {
    pub fn new(gl: Arc<glow::Context>, primitive: IRTopology, index: IRIndexType) -> Option<Self> {
        unsafe {
            let id = gl.create_vertex_array().ok()?;

            debug!("Allocated VBO ID: {:?}", id);
            Some(VertexArray {
                gl,
                id,
                draw_mode: match primitive {
                    IRTopology::Points => glow::POINTS,
                    IRTopology::Lines => glow::LINES,
                    IRTopology::Triangles => glow::TRIANGLES,
                },
                topology_size: match primitive {
                    IRTopology::Points => 1,
                    IRTopology::Lines => 2,
                    IRTopology::Triangles => 3,
                },
                index_type: match index {
                    IRIndexType::U16 => glow::UNSIGNED_SHORT,
                    IRIndexType::U32 => glow::UNSIGNED_INT,
                },
                index_size: match index {
                    IRIndexType::U16 => 2,
                    IRIndexType::U32 => 4,
                },
            })
        }
    }

    #[inline(always)]
    #[must_use]
    pub fn bind(&self) -> VertexArrayBinding<'_> {
        VertexArrayBinding::new(&self.gl, self)
    }

    #[inline(always)]
    pub(crate) fn as_inner(&self) -> glow::VertexArray {
        self.id
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        debug!("Dropping VBO ID: {:?}", self.id);
        unsafe {
            self.gl.delete_vertex_array(self.id);
        }
    }
}
