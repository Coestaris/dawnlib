use yage2_graphics::construct_chain;
use yage2_graphics::pass::{ChainCons, ChainNil};

#[cfg(feature = "gl")]
mod chain_impl {
    use glam::{Vec2, Vec3};
    use yage2_graphics::gl::bindings;
    use yage2_graphics::pass::RenderPass;
    use yage2_graphics::renderable::Renderable;

    pub(crate) struct Pass {}

    impl RenderPass for Pass {
        fn name(&self) -> &str {
            "BasicPass"
        }

        #[inline(always)]
        fn on_renderable(&mut self, renderable: &Renderable) {
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

            unsafe fn draw_quad(a: Vec2, b: Vec2, c: Vec2, d: Vec2) {
                bindings::Begin(bindings::QUADS);
                bindings::Vertex2f(a.x, a.y);
                bindings::Color4f(1.0, 0.0, 0.0, 1.0); // Red color for vertex a
                bindings::Vertex2f(b.x, b.y);
                bindings::Color4f(0.0, 1.0, 0.0, 1.0); // Green color for vertex b
                bindings::Vertex2f(c.x, c.y);
                bindings::Color4f(0.0, 0.0, 1.0, 1.0); // Blue color for vertex c
                bindings::Vertex2f(d.x, d.y);
                bindings::Color4f(1.0, 1.0, 0.0, 1.0); // Yellow color for vertex d
                bindings::End();
            }

            // Draw the quad using the vertices
            unsafe {
                draw_quad(
                    Vec2::new(vertices[0].x, vertices[0].y),
                    Vec2::new(vertices[1].x, vertices[1].y),
                    Vec2::new(vertices[2].x, vertices[2].y),
                    Vec2::new(vertices[3].x, vertices[3].y),
                );
            }
        }
    }
}

use chain_impl::*;

pub(crate) fn construct_chain() -> ChainCons<Pass, ChainNil> {
    construct_chain!(Pass {})
}
