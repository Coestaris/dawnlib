use crate::gl::bindings;
use crate::gl::bindings::types::{GLint, GLsizei, GLuint};
use log::debug;

pub struct VertexArray {
    id: GLuint,
}

type VertexAttributeId = GLuint;

pub enum VertexAttributeFormat {
    Float32,
}

impl VertexAttributeFormat {
    #[inline(always)]
    fn gl_type(&self) -> GLuint {
        match self {
            VertexAttributeFormat::Float32 => bindings::FLOAT,
        }
    }

    #[inline(always)]
    fn size(&self) -> usize {
        match self {
            VertexAttributeFormat::Float32 => 4,
        }
    }
}

pub struct VertexAttribute {
    pub id: VertexAttributeId,
    pub sample_size: usize,
    pub format: VertexAttributeFormat,
    pub stride_samples: usize, // In samples
    pub offset_samples: usize, // In samples
}

pub struct VertexArrayBinding<'a> {
    vertex_array: &'a VertexArray,
}

pub enum DrawElementsMode {
    Points,
    Lines,
    LineStrip,
    LineLoop,
    Triangles,
    TriangleStrip,
    TriangleFan,
}

impl DrawElementsMode {
    #[inline(always)]
    fn gl_type(&self) -> GLuint {
        match self {
            DrawElementsMode::Points => bindings::POINTS,
            DrawElementsMode::Lines => bindings::LINES,
            DrawElementsMode::LineStrip => bindings::LINE_STRIP,
            DrawElementsMode::LineLoop => bindings::LINE_LOOP,
            DrawElementsMode::Triangles => bindings::TRIANGLES,
            DrawElementsMode::TriangleStrip => bindings::TRIANGLE_STRIP,
            DrawElementsMode::TriangleFan => bindings::TRIANGLE_FAN,
        }
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

    pub fn setup_attribute(&self, attribute: VertexAttribute) -> Result<(), String> {
        unsafe {
            bindings::EnableVertexAttribArray(attribute.id);
            bindings::VertexAttribPointer(
                attribute.id,
                attribute.sample_size as GLint,
                attribute.format.gl_type(),
                bindings::FALSE,
                (attribute.stride_samples * attribute.format.size()) as GLsizei,
                (attribute.offset_samples * attribute.format.size()) as *const _,
            );
        }

        Ok(())
    }

    pub fn draw_elements(&self, count: usize, mode: DrawElementsMode) {
        unsafe {
            bindings::DrawElements(
                mode.gl_type(),
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
    pub fn new() -> Result<Self, String> {
        let mut id: GLuint = 0;
        unsafe {
            bindings::GenVertexArrays(1, &mut id);
            if id == 0 {
                return Err("Failed to create VBO".to_string());
            }
        }

        debug!("Allocated VBO ID: {}", id);
        Ok(VertexArray { id })
    }

    #[inline(always)]
    #[must_use]
    pub fn bind(&self) -> VertexArrayBinding<'_> {
        VertexArrayBinding::new(self)
    }

    #[inline(always)]
    fn id(&self) -> GLuint {
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
