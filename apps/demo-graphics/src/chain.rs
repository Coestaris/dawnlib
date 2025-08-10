use glam::{Vec2, Vec3};
use log::info;
use yage2_graphics::construct_chain;
use yage2_graphics::gl::bindings;
use yage2_graphics::passes::chain::{ChainCons, ChainNil};
use yage2_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use yage2_graphics::passes::pipeline::RenderPipeline;
use yage2_graphics::passes::RenderPass;
use yage2_graphics::passes::result::PassExecuteResult;
use yage2_graphics::renderable::Renderable;

#[derive(Clone, Copy, Debug)]
pub(crate) enum PassEvents {
    ChangeColor(Vec3),
}

pub(crate) struct Pass {
    id: RenderPassTargetId,
    color: Vec3,
}

fn dispatch_pass(ptr: *mut u8, event: &PassEvents) {
    let pass = unsafe { &mut *(ptr as *mut Pass) };
    pass.dispatch(event);
}

impl Pass {
    pub fn new() -> Self {
        Pass {
            id: RenderPassTargetId::new(),
            color: Default::default(),
        }
    }

    pub fn get_id(&self) -> RenderPassTargetId {
        self.id
    }

    fn create_event_target(&self) -> PassEventTarget<PassEvents> {
        PassEventTarget::new(dispatch_pass, self.id, self)
    }
}

impl RenderPass<PassEvents> for Pass {
    fn get_target(&self) -> Vec<PassEventTarget<PassEvents>> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &PassEvents) {
        match event {
            PassEvents::ChangeColor(color) => {
                info!("Changing color to: {:?}", color);
                self.color = *color;
            }
        }
    }

    fn name(&self) -> &str {
        "BasicPass"
    }

    #[inline(always)]
    fn on_renderable(&mut self, renderable: &Renderable) -> PassExecuteResult {
        let mut vertices = [
            Vec3::new(-0.5, -0.5, 0.0),
            Vec3::new(0.5, -0.5, 0.0),
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(-0.5, 0.5, 0.0),
        ];
        // Multiply vertices by the model matrix
        let (s, r, t) = renderable.model.to_scale_rotation_translation();
        for vertex in &mut vertices {
            *vertex = *vertex + t;
        }

        unsafe fn draw_quad(a: Vec2, b: Vec2, c: Vec2, d: Vec2, color: Vec3) {
            bindings::Begin(bindings::QUADS);
            bindings::Vertex2f(a.x, a.y);
            bindings::Color3f(color.x, color.y, color.z);
            bindings::Vertex2f(b.x, b.y);
            bindings::Color3f(color.x, color.y, color.z);
            bindings::Vertex2f(c.x, c.y);
            bindings::Color3f(color.x, color.y, color.z);
            bindings::Vertex2f(d.x, d.y);
            bindings::Color3f(color.x, color.y, color.z);
            bindings::End();
        }

        // Draw the quad using the vertices
        unsafe {
            draw_quad(
                Vec2::new(vertices[0].x, vertices[0].y),
                Vec2::new(vertices[1].x, vertices[1].y),
                Vec2::new(vertices[2].x, vertices[2].y),
                Vec2::new(vertices[3].x, vertices[3].y),
                self.color,
            );
        }

        PassExecuteResult::new(1, 1)
    }
}

pub(crate) fn crete_pipeline() -> (
    RenderPipeline<ChainCons<PassEvents, Pass, ChainNil<PassEvents>>, PassEvents>,
    RenderPassTargetId,
) {
    let pass = Pass::new();
    let id = pass.get_id();
    let chain = construct_chain!(pass);
    (RenderPipeline::new(chain), id)
}
