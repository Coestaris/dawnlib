use std::thread::sleep;
use glam::{Vec2, Vec3};
use log::info;
use yage2_graphics::construct_chain;
use yage2_graphics::gl::bindings;
use yage2_graphics::passes::chain::{ChainCons, ChainNil};
use yage2_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use yage2_graphics::passes::pipeline::RenderPipeline;
use yage2_graphics::passes::result::PassExecuteResult;
use yage2_graphics::passes::RenderPass;
use yage2_graphics::renderable::Renderable;

#[derive(Clone, Copy, Debug)]
pub(crate) enum CustomPassEvent {
    ChangeColor(Vec3),
}

pub(crate) struct GeometryPass {
    id: RenderPassTargetId,
    color: Vec3,
}

impl GeometryPass {
    pub fn new(id: RenderPassTargetId) -> Self {
        GeometryPass {
            id,
            color: Vec3::new(1.0, 1.0, 1.0),
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
        }
    }

    fn name(&self) -> &str {
        "GeometryPass"
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

        // Imitate some heavy computation
        sleep(std::time::Duration::from_millis(3));

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
        }
    }

    fn name(&self) -> &str {
        "AABBPass"
    }

    #[inline(always)]
    fn on_renderable(&mut self, renderable: &Renderable) -> PassExecuteResult {
        PassExecuteResult::ok(0, 0)
    }
}
