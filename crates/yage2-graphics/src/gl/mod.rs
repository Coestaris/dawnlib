mod assets;
mod bindings;
mod debug;
mod probe;

use crate::gl::assets::{ShaderAssetFactory, TextureAssetFactory};
use crate::gl::debug::{Debugger, MessageType};
use crate::renderable::Renderable;
use crate::renderer::{
    RendererBackendConfig, RendererBackendError, RendererBackendTrait, RendererTickResult,
};
use crate::view::{ViewError, ViewHandle};
use glam::{Vec2, Vec3, Vec4};
use log::{error, info, warn};
use std::fmt::{Display, Formatter};
use yage2_core::assets::factory::FactoryBinding;

pub struct GLRenderer {
    view_handle: ViewHandle,
    debugger: Debugger,
    texture_factory: Option<TextureAssetFactory>,
    shader_factory: Option<ShaderAssetFactory>,
}
pub struct GLRendererConfig {
    pub fps: usize,
    pub vsync: bool,
    pub texture_factory_binding: Option<FactoryBinding>,
    pub shader_factory_binding: Option<FactoryBinding>,
}

#[derive(Debug, Clone)]
pub enum GLRendererError {
    ViewError(ViewError),
}

// OpenGL has a lot of platform-dependent code,
// so we define a trait for the view handle.
// Bless the Rust for dealing with circular dependencies with such ease.
#[cfg(feature = "gl")]
pub(crate) trait ViewHandleOpenGL {
    fn create_context(&mut self, fps: usize, vsync: bool) -> Result<(), ViewError>;
    fn get_proc_addr(&self, symbol: &str) -> Result<*const std::ffi::c_void, ViewError>;
    fn swap_buffers(&self) -> Result<(), ViewError>;
}

impl Display for GLRendererError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred in the graphics module")
    }
}

impl std::error::Error for GLRendererError {}

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

// Texture and shader assets cannot be handled from the ECS (like other assets),
// because they are tightly coupled with the OpenGL context and cannot be
// loaded asynchronously.
// So OpenGL renderer handles events for these assets on each draw tick.
impl RendererBackendTrait for GLRenderer {
    fn new(
        cfg: RendererBackendConfig,
        mut view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>
    where
        Self: Sized,
    {
        // Create the OpenGL context
        view_handle.create_context(cfg.fps, cfg.vsync).unwrap();
        // Load OpenGL functions using the OS-specific loaders
        bindings::load_with(|symbol| {
            view_handle
                .get_proc_addr(symbol)
                .expect("Failed to load OpenGL function")
        });

        // Stat the OpenGL context
        let version = unsafe { probe::get_version() };
        if let Some(version) = version {
            info!("OpenGL version: {}", version);
        } else {
            warn!("Failed to get OpenGL version. This may cause issues with rendering.");
        }
        let renderer = unsafe { probe::get_renderer() };
        if let Some(renderer) = renderer {
            info!("OpenGL renderer: {}", renderer);
        } else {
            warn!("Failed to get OpenGL renderer. This may cause issues with rendering.");
        }
        let vendor = unsafe { probe::get_vendor() };
        if let Some(vendor) = vendor {
            info!("OpenGL vendor: {}", vendor);
        } else {
            warn!("Failed to get OpenGL vendor. This may cause issues with rendering.");
        }

        // Setup factories for texture and shader assets
        // These factories are used to load and manage texture and shader assets.
        let texture_factory = if let Some(binding) = cfg.texture_factory_binding {
            let mut factory = TextureAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };
        let shader_factory = if let Some(binding) = cfg.shader_factory_binding {
            let mut factory = ShaderAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };

        Ok(GLRenderer {
            view_handle,
            texture_factory,
            shader_factory,
            debugger: unsafe {
                Debugger::new(|source, rtype, severity, message| match rtype {
                    MessageType::Error => {
                        error!("OpenGL: {}: {}: {}", source, severity, message);
                    }
                    MessageType::DeprecatedBehavior | MessageType::UndefinedBehavior => {
                        warn!("OpenGL: {}: {}: {}", source, severity, message);
                    }
                    _ => {
                        info!("OpenGL: {}: {}: {}", source, severity, message);
                    }
                })
            },
        })
    }

    fn tick(
        &mut self,
        renderables: &[Renderable],
    ) -> Result<RendererTickResult, RendererBackendError> {
        // Process events asset factories
        if let Some(factory) = &mut self.texture_factory {
            factory.process_events();
        }
        if let Some(factory) = &mut self.shader_factory {
            factory.process_events();
        }
        unsafe {
            bindings::ClearColor(0.0, 0.2, 0.0, 1.0);
            bindings::Clear(bindings::COLOR_BUFFER_BIT);

            for renderable in renderables {
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

                // Draw the quad using the vertices
                draw_quad(
                    Vec2::new(vertices[0].x, vertices[0].y),
                    Vec2::new(vertices[1].x, vertices[1].y),
                    Vec2::new(vertices[2].x, vertices[2].y),
                    Vec2::new(vertices[3].x, vertices[3].y),
                );
            }
        }

        self.view_handle
            .swap_buffers()
            .map_err(GLRendererError::ViewError)?;

        Ok(RendererTickResult {
            draw_calls: renderables.len() * 4, // 4 vertices per quad
            drawn_primitives: renderables.len(),
        })
    }
}
