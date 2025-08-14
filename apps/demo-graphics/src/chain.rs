use glam::{Mat4, Vec3};
use log::info;
use yage2_core::assets::TypedAsset;
use yage2_graphics::gl::bindings;
use yage2_graphics::gl::entities::{
    Buffer, BufferType, BufferUsage, DrawElementsMode, ShaderProgram, UniformLocation, VertexArray,
    VertexAttribute, VertexAttributeFormat,
};
use yage2_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use yage2_graphics::passes::result::PassExecuteResult;
use yage2_graphics::passes::RenderPass;
use yage2_graphics::renderable::Renderable;
use yage2_graphics::renderer::RendererBackend;

pub struct Mesh {
    vao: VertexArray,
    vbo: Buffer,
    ebo: Buffer,
    count: usize,
}

pub fn create_quad() -> Mesh {
    let vertices: [f32; 12] = [
        0.5, 0.5, 0.0, // top right
        0.5, -0.5, 0.0, // bottom right
        -0.5, -0.5, 0.0, // bottom letf
        -0.5, 0.5, 0.0, // top left
    ];
    let indices: [u32; 6] = [
        // note that we start from 0!
        0, 1, 3, // first Triangle
        1, 2, 3, // second Triangle
    ];

    let vao = VertexArray::new().unwrap();
    let mut vbo = Buffer::new(BufferType::ArrayBuffer).unwrap();
    let mut ebo = Buffer::new(BufferType::ElementArrayBuffer).unwrap();

    let vao_binding = vao.bind();
    let vbo_binding = vbo.bind();
    let ebo_binding = ebo.bind();

    vbo_binding
        .feed(&vertices, BufferUsage::StaticDraw)
        .unwrap();
    ebo_binding.feed(&indices, BufferUsage::StaticDraw).unwrap();

    vao.setup_attribute(VertexAttribute {
        id: 0,
        sample_size: 3,
        format: VertexAttributeFormat::Float32,
        stride_samples: 3,
        offset_samples: 0,
    })
    .unwrap();

    drop(vbo_binding);
    drop(ebo_binding);
    drop(vao_binding);

    Mesh {
        vao,
        vbo,
        ebo,
        count: 6,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    UpdateShader(TypedAsset<ShaderProgram>),
    ChangeColor(Vec3),
}

struct TriangleShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
}

pub(crate) struct GeometryPass {
    id: RenderPassTargetId,
    shader: Option<TriangleShaderContainer>,
    color: Vec3,
    mesh: Mesh,
}

impl GeometryPass {
    pub fn new(id: RenderPassTargetId, mesh: Mesh) -> Self {
        GeometryPass {
            id,
            shader: None,
            color: Vec3::new(1.0, 1.0, 1.0),
            mesh,
        }
    }
}

impl RenderPass<CustomPassEvent> for GeometryPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_geometry_pass(ptr: *mut u8, event: &CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut GeometryPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_geometry_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: &CustomPassEvent) {
        match event {
            CustomPassEvent::ChangeColor(color) => {
                info!("Changing color to: {:?}", color);
                self.color = *color;
            }
            CustomPassEvent::UpdateShader(shader) => {
                info!("Updating shader: {:?}", shader);
                let clone = shader.clone();
                self.shader = Some(TriangleShaderContainer {
                    shader: clone,
                    model_location: shader.cast().get_uniform_location("model").unwrap(),
                    view_location: shader.cast().get_uniform_location("view").unwrap(),
                    proj_location: shader.cast().get_uniform_location("projection").unwrap(),
                });
            }
        }
    }

    fn name(&self) -> &str {
        "GeometryPass"
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> PassExecuteResult {
        // Nothing to do if no shader is set
        if self.shader.is_none() {
            return PassExecuteResult::default();
        }
        let container = self.shader.as_ref().unwrap();
        let shader = container.shader.cast();

        let shader_use = shader.use_program();
        shader_use.set_uniform(container.model_location, renderable.model);
        shader_use.set_uniform(container.view_location, Mat4::IDENTITY);
        shader_use.set_uniform(container.proj_location, Mat4::IDENTITY);

        let binding = self.mesh.vao.bind();
        binding.draw_elements(self.mesh.count, DrawElementsMode::Triangles);

        PassExecuteResult::ok(1, 1)
    }
}

pub(crate) struct AABBPass {
    id: RenderPassTargetId,
    color: Vec3,
}
impl AABBPass {
    pub fn new(id: RenderPassTargetId) -> Self {
        AABBPass {
            id,
            color: Default::default(),
        }
    }
}

impl RenderPass<CustomPassEvent> for AABBPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_aabb_pass(ptr: *mut u8, event: &CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut AABBPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_aabb_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: &CustomPassEvent) {
        match event {
            CustomPassEvent::ChangeColor(color) => {
                info!("Changing color to: {:?}", color);
                self.color = *color;
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "AABBPass"
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> PassExecuteResult {
        PassExecuteResult::ok(0, 0)
    }
}
