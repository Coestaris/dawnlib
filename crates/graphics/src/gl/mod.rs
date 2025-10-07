pub mod assets;
#[cfg(not(target_arch = "wasm32"))]
pub mod context_glutin;
#[cfg(target_arch = "wasm32")]
pub mod context_webgl;
mod debug;
pub mod font;
pub mod material;
pub mod mesh;
pub mod probe;
pub mod raii;
pub mod timer;

use crate::gl::assets::{
    FontAssetFactory, MaterialAssetFactory, MeshAssetFactory, ShaderAssetFactory,
    TextureAssetFactory,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::gl::context_glutin::{Context, ContextError};
#[cfg(target_arch = "wasm32")]
use crate::gl::context_webgl::{Context, ContextError};
use crate::gl::probe::OpenGLInfo;
use crate::passes::events::PassEventTrait;
use crate::renderer::backend::{RendererBackendError, RendererBackendTrait, RendererConfig};
use dawn_assets::factory::FactoryBinding;
use glam::UVec2;
use log::error;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

pub struct GLRenderer<E: PassEventTrait> {
    _marker: std::marker::PhantomData<E>,

    context: Context,

    pub info: OpenGLInfo,
    pub gl: Arc<glow::Context>,

    // Factories for texture and shader assets
    texture_factory: Option<TextureAssetFactory>,
    shader_factory: Option<ShaderAssetFactory>,
    mesh_factory: Option<MeshAssetFactory>,
    material_factory: Option<MaterialAssetFactory>,
    font_factory: Option<FontAssetFactory>,
}

#[derive(Clone)]
pub struct GLRendererConfig {
    pub shader_defines: HashMap<String, String>,
    pub texture_factory_binding: Option<FactoryBinding>,
    pub shader_factory_binding: Option<FactoryBinding>,
    pub mesh_factory_binding: Option<FactoryBinding>,
    pub material_factory_binding: Option<FactoryBinding>,
    pub font_factory_binding: Option<FactoryBinding>,
}

#[derive(Debug, Error)]
pub enum GLRendererError {
    #[error("Failed to create OpenGL context: {0}")]
    ContextCreateError(#[from] ContextError),
}

// Texture and shader assets cannot be handled from the ECS (like other assets),
// because they are tightly coupled with the OpenGL context and cannot be
// loaded asynchronously.
// So OpenGL renderer handles events for these assets on each draw tick.
impl<E: PassEventTrait> RendererBackendTrait<E> for GLRenderer<E> {
    fn new(cfg: RendererConfig, context: Context) -> Result<Self, RendererBackendError>
    where
        Self: Sized,
    {
        unsafe {
            // Create main OpenGL context
            let gl = context.glow()?;

            // Stat the OpenGL context
            let info = OpenGLInfo::new(&gl);

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
                let mut factory = ShaderAssetFactory::new(cfg.shader_defines);
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

            Ok(GLRenderer::<E> {
                _marker: Default::default(),
                context,
                texture_factory,
                shader_factory,
                mesh_factory,
                material_factory,
                font_factory,
                gl,
                info,
            })
        }
    }

    #[inline(always)]
    fn before_frame(&mut self) -> Result<(), RendererBackendError> {
        // Process events asset factories
        if let Some(factory) = &mut self.texture_factory {
            factory.process_events::<E>(&self.gl);
        }
        if let Some(factory) = &mut self.shader_factory {
            factory.process_events::<E>(&self.gl);
        }
        if let Some(factory) = &mut self.mesh_factory {
            factory.process_events::<E>(&self.gl);
        }
        if let Some(factory) = &mut self.material_factory {
            factory.process_events::<E>(&self.gl);
        }
        if let Some(factory) = &mut self.font_factory {
            factory.process_events::<E>(&self.gl);
        }

        // User will handle clearing the screen in the render passes.

        Ok(())
    }

    #[inline(always)]
    fn after_frame(&mut self) -> Result<(), RendererBackendError> {
        self.context.swap_buffers();

        Ok(())
    }

    fn resize(&self, size: UVec2) -> Result<(), RendererBackendError> {
        Ok(self.context.resize(size))
    }
}
