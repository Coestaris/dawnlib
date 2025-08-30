pub mod assets;
pub mod bindings;
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
use crate::gl::debug::{Debugger, MessageType};
use crate::passes::events::PassEventTrait;
use crate::renderer::backend::{RendererBackendConfig, RendererBackendError, RendererBackendTrait};
use crate::view::{ViewError, ViewHandle};
use dawn_assets::factory::FactoryBinding;
use log::{debug, error, info, warn};
use std::fmt::{Display, Formatter};

pub struct GLRenderer<E: PassEventTrait> {
    _marker: std::marker::PhantomData<E>,

    view_handle: ViewHandle,
    _debugger: Debugger,

    // Factories for texture and shader assets
    texture_factory: Option<TextureAssetFactory>,
    shader_factory: Option<ShaderAssetFactory>,
    mesh_factory: Option<MeshAssetFactory>,
    material_factory: Option<MaterialAssetFactory>,
    font_factory: Option<FontAssetFactory>,
}

pub struct GLRendererConfig {
    // pub fps: usize,
    // pub vsync: bool,
    pub texture_factory_binding: Option<FactoryBinding>,
    pub shader_factory_binding: Option<FactoryBinding>,
    pub mesh_factory_binding: Option<FactoryBinding>,
    pub material_factory_binding: Option<FactoryBinding>,
    pub font_factory_binding: Option<FactoryBinding>,
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
    fn get_proc_addr(&mut self, symbol: &str) -> Result<*const std::ffi::c_void, ViewError>;
    fn swap_buffers(&self) -> Result<(), ViewError>;
}

impl Display for GLRendererError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred in the graphics module")
    }
}

impl std::error::Error for GLRendererError {}

fn stat_opengl_context() {
    let version = unsafe { probe::get_version() };
    info!("OpenGL stat");
    if let Some(version) = version {
        info!("  Version: {}", version);
    } else {
        warn!("Failed to get OpenGL version. This may cause issues with rendering.");
    }
    let renderer = unsafe { probe::get_renderer() };
    if let Some(renderer) = renderer {
        info!("  Renderer: {}", renderer);
    } else {
        warn!("Failed to get OpenGL renderer. This may cause issues with rendering.");
    }
    let vendor = unsafe { probe::get_vendor() };
    if let Some(vendor) = vendor {
        info!("  Vendor: {}", vendor);
    } else {
        warn!("Failed to get OpenGL vendor. This may cause issues with rendering.");
    }
    let shading_language_version = unsafe { probe::get_shading_language_version() };
    if let Some(version) = shading_language_version {
        info!("  GLSL version: {}", version);
    } else {
        warn!(
            "Failed to get OpenGL shading language version. This may cause issues with rendering."
        );
    }
    let binary_formats = unsafe { probe::get_binary_formats() };
    if !binary_formats.is_empty() {
        info!("  Binary formats: {:?}", binary_formats);
    } else {
        warn!("Failed to get OpenGL binary formats. This may cause issues with rendering.");
    }
    // let extensions = unsafe { probe::get_extensions() };
    // if !extensions.is_empty() {
    //     debug!("OpenGL extensions: {:?}", extensions);
    // } else {
    //     warn!("Failed to get OpenGL extensions. This may cause issues with rendering.");
    // }
}

// Texture and shader assets cannot be handled from the ECS (like other assets),
// because they are tightly coupled with the OpenGL context and cannot be
// loaded asynchronously.
// So OpenGL renderer handles events for these assets on each draw tick.
impl<E: PassEventTrait> RendererBackendTrait<E> for GLRenderer<E> {
    fn new(
        cfg: RendererBackendConfig,
        mut view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>
    where
        Self: Sized,
    {
        // Create the OpenGL context
        view_handle.create_context(0, false).unwrap();
        // Load OpenGL functions using the OS-specific loaders
        bindings::load_with(|symbol| {
            view_handle
                .get_proc_addr(symbol)
                .expect("Failed to load OpenGL function")
        });

        // Stat the OpenGL context
        stat_opengl_context();

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
            view_handle,
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
        self.view_handle
            .swap_buffers()
            .map_err(GLRendererError::ViewError)?;

        Ok(())
    }
}
