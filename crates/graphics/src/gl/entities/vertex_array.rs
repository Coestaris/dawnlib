use crate::gl::bindings;
use crate::gl::bindings::types::{GLint, GLsizei, GLuint};
use dawn_assets::ir::mesh::{IRMeshLayout, IRMeshLayoutSampleType, IRPrimitive};
use log::debug;

pub struct VertexArray {
    id: GLuint,
    draw_mode: GLuint,
}

pub struct VertexArrayBinding<'a> {
    vertex_array: &'a VertexArray,
}

fn primitive_gl_type(primitive: IRPrimitive) -> GLuint {
    match primitive {
        IRPrimitive::Points => bindings::POINTS,
        IRPrimitive::Lines => bindings::LINES,
        IRPrimitive::LineStrip => bindings::LINE_STRIP,
        IRPrimitive::LineLoop => bindings::LINE_LOOP,
        IRPrimitive::Triangles => bindings::TRIANGLES,
        IRPrimitive::TriangleStrip => bindings::TRIANGLE_STRIP,
        IRPrimitive::TriangleFan => bindings::TRIANGLE_FAN,
    }
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

    pub fn draw_elements(&self, count: usize) {
        unsafe {
            bindings::DrawElements(
                self.vertex_array.draw_mode,
                count as GLsizei,
                bindings::UNSIGNED_INT,
                std::ptr::null(),
            );
        }
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
    pub fn new(primitive: IRPrimitive) -> Result<Self, String> {
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
            draw_mode: primitive_gl_type(primitive),
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
