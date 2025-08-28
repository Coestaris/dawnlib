use crate::gl::bindings;
use crate::gl::bindings::types::{GLint, GLsizei, GLuint};
use crate::passes::result::PassExecuteResult;
use dawn_assets::ir::mesh::{IRIndexType, IRMeshLayout, IRMeshLayoutSampleType, IRTopology};
use log::debug;

pub struct VertexArray {
    id: GLuint,
    draw_mode: GLuint,
    topology_size: usize,
    index_type: GLuint,
    index_size: usize,
}

pub struct VertexArrayBinding<'a> {
    vertex_array: &'a VertexArray,
}

impl<'a> VertexArrayBinding<'a> {
    #[inline(always)]
    fn new(vertex_array: &'a VertexArray) -> Self {
        unsafe {
            bindings::BindVertexArray(vertex_array.id());
        }
        Self { vertex_array }
    }

    pub fn setup_attribute(&self, index: usize, attribute: &IRMeshLayout) -> Result<(), String> {
        let gl_format = match attribute.sample_type {
            IRMeshLayoutSampleType::Float => bindings::FLOAT,
            IRMeshLayoutSampleType::U32 => bindings::UNSIGNED_INT,
        };
        unsafe {
            bindings::EnableVertexAttribArray(index as GLuint);
            bindings::VertexAttribPointer(
                index as GLuint,
                attribute.samples as GLint,
                gl_format,
                bindings::FALSE,
                attribute.stride_bytes as GLsizei,
                attribute.offset_bytes as *const _,
            );
        }

        Ok(())
    }

    #[inline(always)]
    pub fn draw_elements(&self, count: usize, offset: usize) -> PassExecuteResult {
        unsafe {
            bindings::DrawElements(
                self.vertex_array.draw_mode,
                count as GLsizei,
                self.vertex_array.index_type,
                (offset * self.vertex_array.index_size) as *const _,
            );
        }

        PassExecuteResult::ok(1, count / self.vertex_array.topology_size)
    }
}

impl<'a> Drop for VertexArrayBinding<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            bindings::BindVertexArray(0);
        }
    }
}

impl VertexArray {
    pub fn new(primitive: IRTopology, index: IRIndexType) -> Result<Self, String> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenVertexArrays(1, &mut id);
            if id == 0 {
                return Err("Failed to create VBO".to_string());
            }
        }

        debug!("Allocated VBO ID: {}", id);
        Ok(VertexArray {
            id,
            draw_mode: match primitive {
                IRTopology::Points => bindings::POINTS,
                IRTopology::Lines => bindings::LINES,
                IRTopology::Triangles => bindings::TRIANGLES,
            },
            topology_size: match primitive {
                IRTopology::Points => 1,
                IRTopology::Lines => 2,
                IRTopology::Triangles => 3,
            },
            index_type: match index {
                IRIndexType::U16 => bindings::UNSIGNED_SHORT,
                IRIndexType::U32 => bindings::UNSIGNED_INT,
            },
            index_size: match index {
                IRIndexType::U16 => 2,
                IRIndexType::U32 => 4,
            },
        })
    }

    #[inline(always)]
    #[must_use]
    pub fn bind(&self) -> VertexArrayBinding<'_> {
        VertexArrayBinding::new(self)
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        debug!("Dropping VBO ID: {}", self.id);
        unsafe {
            bindings::DeleteVertexArrays(1, &self.id);
        }
    }
}
