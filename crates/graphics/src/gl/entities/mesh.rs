use crate::gl::bindings;
use crate::gl::bindings::types::GLuint;
use crate::gl::entities::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use crate::gl::entities::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use crate::gl::entities::vertex_array::VertexArray;
use crate::passes::result::PassExecuteResult;
use dawn_assets::ir::mesh::{IRMesh, IRPrimitive, IRVertex};
use dawn_assets::{AssetCastable, AssetMemoryUsage};
use glam::Vec3;

pub struct Mesh {
    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
    pub ebo: ElementArrayBuffer,
    pub indices_count: usize,
    pub primitives_count: usize,
    pub min: Vec3,
    pub max: Vec3,
}

impl AssetCastable for Mesh {}

impl Mesh {
    pub fn from_ir(ir: IRMesh) -> Result<(Self, AssetMemoryUsage), String> {
        let vao = VertexArray::new(ir.primitive.clone())
            .map_err(|e| format!("Failed to create VertexArray: {}", e))?;
        let mut vbo =
            ArrayBuffer::new().map_err(|e| format!("Failed to create ArrayBuffer: {}", e))?;
        let mut ebo = ElementArrayBuffer::new()
            .map_err(|e| format!("Failed to create ElementArrayBuffer: {}", e))?;

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        vbo_binding
            .feed(ir.raw_vertices(), ArrayBufferUsage::StaticDraw)
            .map_err(|e| format!("Failed to feed vertices to ArrayBuffer: {}", e))?;
        ebo_binding
            .feed(ir.raw_indices(), ElementArrayBufferUsage::StaticDraw)
            .map_err(|e| format!("Failed to feed indices to ElementArrayBuffer: {}", e))?;

        for (i, layout) in IRVertex::layout().iter().enumerate() {
            vao_binding
                .setup_attribute(i, layout)
                .map_err(|e| format!("Failed to enable attribute in VertexArray: {}", e))?;
        }

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        Ok((
            Mesh {
                vao,
                vbo,
                ebo,
                indices_count: ir.indices.len(),
                primitives_count: ir.primitives_count,
                min: ir.bounds.min(),
                max: ir.bounds.max(),
            },
            AssetMemoryUsage::new(size_of::<Mesh>(), 0),
        ))
    }

    #[inline(always)]
    pub fn draw(&self) -> PassExecuteResult {
        let binding = self.vao.bind();
        binding.draw_elements(self.indices_count);
        PassExecuteResult::ok(1, self.primitives_count)
    }
}
