mod assets;
pub mod bindings;
mod debug;
mod probe;

use crate::gl::assets::{ShaderAssetFactory, TextureAssetFactory};
use crate::gl::debug::{Debugger, MessageType};
use crate::renderer::{RendererBackendConfig, RendererBackendError, RendererBackendTrait};
use crate::view::{ViewError, ViewHandle};
use log::{debug, error, info, warn};
use std::fmt::{Display, Formatter};
use yage2_core::assets::factory::FactoryBinding;

pub struct GLRenderer<E>
where
    E: Copy + 'static,
{
    _marker: std::marker::PhantomData<E>,

    view_handle: ViewHandle,
    debugger: Debugger,

    // Factories for texture and shader assets
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

// Texture and shader assets cannot be handled from the ECS (like other assets),
// because they are tightly coupled with the OpenGL context and cannot be
// loaded asynchronously.
// So OpenGL renderer handles events for these assets on each draw tick.
impl<E> RendererBackendTrait<E> for GLRenderer<E>
where
    E: Copy + 'static,
{
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
        let shading_language_version = unsafe { probe::get_shading_language_version() };
        if let Some(version) = shading_language_version {
            info!("OpenGL shading language version: {}", version);
        } else {
            warn!("Failed to get OpenGL shading language version. This may cause issues with rendering.");
        }
        let binary_formats = unsafe { probe::get_binary_formats() };
        if !binary_formats.is_empty() {
            info!("OpenGL binary formats: {:?}", binary_formats);
        } else {
            warn!("Failed to get OpenGL binary formats. This may cause issues with rendering.");
        }
        let extensions = unsafe { probe::get_extensions() };
        if !extensions.is_empty() {
            debug!("OpenGL extensions: {:?}", extensions);
        } else {
            warn!("Failed to get OpenGL extensions. This may cause issues with rendering.");
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

        Ok(GLRenderer::<E> {
            _marker: Default::default(),
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

    #[inline(always)]
    fn before_frame(&mut self) -> Result<(), RendererBackendError> {
        // Process events asset factories
        if let Some(factory) = &mut self.texture_factory {
            factory.process_events();
        }
        if let Some(factory) = &mut self.shader_factory {
            factory.process_events();
        }

        // Clear the screen with a green color
        unsafe {
            bindings::ClearColor(0.0, 0.2, 0.0, 1.0);
            bindings::Clear(bindings::COLOR_BUFFER_BIT);
        }

        Ok(())
    }

    #[inline(always)]
    fn after_frame(&mut self) -> Result<(), RendererBackendError> {
        self.view_handle
            .swap_buffers()
            .map_err(GLRendererError::ViewError)?;

        Ok(())
    }
}
