pub mod assets;
pub mod bindings;
mod context;
mod debug;
pub mod font;
pub mod material;
pub mod mesh;
mod probe;
pub mod raii;

use crate::gl::assets::{
    FontAssetFactory, MaterialAssetFactory, MeshAssetFactory, ShaderAssetFactory,
    TextureAssetFactory,
};
use crate::gl::context::Context;
use crate::gl::debug::{Debugger, MessageType};
use crate::passes::events::PassEventTrait;
use crate::renderer::backend::{RendererBackendError, RendererBackendTrait, RendererConfig};
use dawn_assets::factory::FactoryBinding;
use log::{error, info, warn};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub struct GLRenderer<E: PassEventTrait> {
    _marker: std::marker::PhantomData<E>,

    context: Context,

    _debugger: Debugger,

    // Factories for texture and shader assets
    texture_factory: Option<TextureAssetFactory>,
    shader_factory: Option<ShaderAssetFactory>,
    mesh_factory: Option<MeshAssetFactory>,
    material_factory: Option<MaterialAssetFactory>,
    font_factory: Option<FontAssetFactory>,
}

#[derive(Clone)]
pub struct GLRendererConfig {
    // pub fps: usize,
    // pub vsync: bool,
    pub texture_factory_binding: Option<FactoryBinding>,
    pub shader_factory_binding: Option<FactoryBinding>,
    pub mesh_factory_binding: Option<FactoryBinding>,
    pub material_factory_binding: Option<FactoryBinding>,
    pub font_factory_binding: Option<FactoryBinding>,
}

#[derive(Debug, Error)]
pub enum GLRendererError {
    #[error("Failed to create OpenGL context: {0}")]
    ContextCreateError(#[from] anyhow::Error),
}

unsafe fn stat_opengl_context() {
    info!("OpenGL information:");
    probe::get_version().map_or_else(
        || warn!("Failed to get OpenGL version"),
        |v| info!("OpenGL version: {}.{}", v.major, v.minor),
    );
    probe::get_renderer().map_or_else(
        || warn!("Failed to get OpenGL renderer"),
        |r| info!("  Renderer: {}", r),
    );
    probe::get_vendor().map_or_else(
        || warn!("Failed to get OpenGL vendor"),
        |v| info!("  Vendor: {}", v),
    );
    probe::get_shading_language_version().map_or_else(
        || warn!("Failed to get OpenGL shading language version"),
        |v| info!("  GLSL version: {}", v),
    );
    probe::get_depth_bits().map_or_else(
        || warn!("Failed to get OpenGL depth bits"),
        |b| info!("  Depth bits: {}", b),
    );
}

// Texture and shader assets cannot be handled from the ECS (like other assets),
// because they are tightly coupled with the OpenGL context and cannot be
// loaded asynchronously.
// So OpenGL renderer handles events for these assets on each draw tick.
impl<E: PassEventTrait> RendererBackendTrait<E> for GLRenderer<E> {
    fn new(
        cfg: RendererConfig,
        raw_window: RawWindowHandle,
        raw_display: RawDisplayHandle,
    ) -> Result<Self, RendererBackendError>
    where
        Self: Sized,
    {
        // Create the OpenGL context
        let mut context = Context::new(raw_window, raw_display)?;

        // Load OpenGL functions using the OS-specific loaders
        bindings::load_with(|symbol| {
            // Warn if the symbol is not found
            match context.load_fn(symbol) {
                Ok(addr) => addr,
                Err(e) => {
                    // That's not a catastrophic, but we should know about it
                    warn!("Failed to load OpenGL symbol: {}: {}", symbol, e);
                    std::ptr::null()
                }
            }
        });

        // Stat the OpenGL context
        unsafe {
            stat_opengl_context();
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
        let mesh_factory = if let Some(binding) = cfg.mesh_factory_binding {
            let mut factory = MeshAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };
        let material_factory = if let Some(binding) = cfg.material_factory_binding {
            let mut factory = MaterialAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };
        let font_factory = if let Some(binding) = cfg.font_factory_binding {
            let mut factory = FontAssetFactory::new();
            factory.bind(binding);
            Some(factory)
        } else {
            None
        };

        // Setup the debug output for OpenGL.
        let debugger = Debugger::new(|source, rtype, severity, message| match rtype {
            MessageType::Error => {
                error!("OpenGL: {}: {}: {}", source, severity, message);
            }
            MessageType::DeprecatedBehavior | MessageType::UndefinedBehavior => {
                warn!("OpenGL: {}: {}: {}", source, severity, message);
            }
            _ => {
                info!("OpenGL: {}: {}: {}", source, severity, message);
            }
        });

        Ok(GLRenderer::<E> {
            _marker: Default::default(),
            _debugger: debugger,
            context,
            texture_factory,
            shader_factory,
            mesh_factory,
            material_factory,
            font_factory,
        })
    }

    #[inline(always)]
    fn before_frame(&mut self) -> Result<(), RendererBackendError> {
        // Process events asset factories
        if let Some(factory) = &mut self.texture_factory {
            factory.process_events::<E>();
        }
        if let Some(factory) = &mut self.shader_factory {
            factory.process_events::<E>();
        }
        if let Some(factory) = &mut self.mesh_factory {
            factory.process_events::<E>();
        }
        if let Some(factory) = &mut self.material_factory {
            factory.process_events::<E>();
        }
        if let Some(factory) = &mut self.font_factory {
            factory.process_events::<E>();
        }

        // User will handle clearing the screen in the render passes.

        Ok(())
    }

    #[inline(always)]
    fn after_frame(&mut self) -> Result<(), RendererBackendError> {
        self.context.swap_buffers();

        Ok(())
    }
}
