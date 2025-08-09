mod assets;
mod bindings;

use crate::gl::assets::TextureAssetFactory;
use crate::renderable::Renderable;
use crate::renderer::{
    RendererBackendConfig, RendererBackendError, RendererBackendTrait, RendererTickResult,
};
use crate::view::{ViewError, ViewHandle};
use std::fmt::{Display, Formatter};
use yage2_core::assets::factory::FactoryBinding;

pub struct GLRenderer {
    view_handle: ViewHandle,
    texture_factory: Option<TextureAssetFactory>,
    shader_factory: Option<TextureAssetFactory>,
}
pub struct GLRendererConfig {
    pub texture_factory_binding: Option<FactoryBinding>,
    pub shader_factory_binding: Option<FactoryBinding>,
}

#[derive(Debug, Clone)]
pub enum GLRendererError {}

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
        view_handle.create_context(cfg.fps, cfg.vsync).unwrap();
        bindings::load_with(|symbol| {
            view_handle
                .get_proc_addr(symbol)
                .expect("Failed to load OpenGL function")
        });

        let texture_factory = if let Some(binding) = cfg.texture_factory_binding {
            let mut factory = TextureAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };
        let shader_factory = if let Some(binding) = cfg.shader_factory_binding {
            let mut factory = TextureAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };

        Ok(GLRenderer {
            view_handle,
            texture_factory,
            shader_factory,
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
        }

        self.view_handle.swap_buffers().unwrap();

        Ok(RendererTickResult {
            draw_calls: 0,
            drawn_primitives: 0,
        })
    }
}
