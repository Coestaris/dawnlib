use glam::UVec2;
use glutin::config::{ColorBufferType, Config, ConfigTemplateBuilder};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, NotCurrentGlContext,
    PossiblyCurrentContext, PossiblyCurrentGlContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlConfig;
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{debug, info, warn};
use std::error::Error;
use std::ffi::c_void;
use std::num::NonZeroU32;
use thiserror::Error;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{HandleError, HasWindowHandle};
use winit::window::{Window, WindowAttributes};

pub struct Context {
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Failed to create OpenGL context: {0}, {1}, {2}")]
    ContextCreationError(
        glutin::error::Error,
        glutin::error::Error,
        glutin::error::Error,
    ),
    #[error("Failed to create window with OpenGL context: {0}")]
    WindowCreationError(#[from] Box<dyn Error>),
    #[error("Failed to create window with OpenGL context")]
    WindowResultNone,
    #[error("Failed to build surface attributes: {0}")]
    SurfaceAttributesError(#[from] HandleError),
    #[error("Failed to create OpenGL surface: {0}")]
    SurfaceCreationError(glutin::error::Error),
    #[error("Failed to make context current: {0}")]
    MakeCurrentError(glutin::error::Error),
}

impl Context {
    fn create_gl_context(
        window: &Window,
        gl_config: &Config,
    ) -> Result<NotCurrentContext, ContextError> {
        let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

        // The context creation part. 4.1 is the minimum version for macOS.
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 1))))
            .build(raw_window_handle);

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);

        // There are also some old devices that support neither modern OpenGL nor GLES.
        // To support these we can try and create a 2.1 context.
        let legacy_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
            .build(raw_window_handle);

        // Reuse the uncurrented context from a suspended() call if it exists, otherwise
        // this is the first time resumed() is called, where the context still
        // has to be created.
        let gl_display = gl_config.display();

        unsafe {
            info!("Trying to create modern OpenGL context");
            let err1 = match gl_display.create_context(gl_config, &context_attributes) {
                Ok(context) => return Ok(context),
                Err(e) => e,
            };

            info!("Failed to create modern OpenGL context: {err1}. Trying GLES context");
            let err2 = match gl_display.create_context(gl_config, &fallback_context_attributes) {
                Ok(context) => return Ok(context),
                Err(e) => e,
            };

            info!("Failed to create GLES context: {err2}. Trying legacy OpenGL context");
            match gl_display.create_context(gl_config, &legacy_context_attributes) {
                Ok(context) => return Ok(context),
                Err(e) => Err(ContextError::ContextCreationError(err1, err2, e)),
            }
        }
    }

    // Find the config with the maximum number of samples
    pub fn config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
        configs
            .reduce(|accum, config| {
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                if transparency_check || config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .unwrap()
    }

    pub fn create_contextual_window(
        attributes: WindowAttributes,
        event_loop: &ActiveEventLoop,
    ) -> Result<(Window, Context), ContextError> {
        let template = ConfigTemplateBuilder::new()
            .with_depth_size(24)
            .with_stencil_size(8)
            .with_transparency(false)
            .with_alpha_size(0)
            .with_single_buffering(false)
            .with_buffer_type(ColorBufferType::Rgb {
                r_size: 8,
                g_size: 8,
                b_size: 8,
            });

        debug!(
            "Creating window with OpenGL context. Template: {:?}, Attributes: {:?}",
            template, attributes
        );
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(attributes));
        let (window, config) = display_builder.build(event_loop, template, Self::config_picker)?;
        if window.is_none() {
            return Err(ContextError::WindowResultNone);
        }
        let window = window.unwrap();

        debug!("Creating OpenGL context. Config: {:?}", config);
        let context = Self::create_gl_context(&window, &config)?.treat_as_possibly_current();

        let surface_attrs = window.build_surface_attributes(Default::default())?;
        debug!("Creating OpenGL surface. Attributes: {:?}", surface_attrs);
        let surface = unsafe {
            config
                .display()
                .create_window_surface(&config, &surface_attrs)
                .map_err(ContextError::SurfaceCreationError)?
        };

        debug!("Making context current");
        (&context)
            .make_current(&surface)
            .map_err(ContextError::MakeCurrentError)?;

        // Try setting vsync.
        if let Err(res) =
            surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
        {
            warn!("Failed to set vsync: {res}");
        }

        Ok((window, Self { context, surface }))
    }

    pub fn resize(&self, size: UVec2) {
        let (w, h) = (
            NonZeroU32::new(size.x).unwrap(),
            NonZeroU32::new(size.y).unwrap(),
        );

        self.surface.resize(&self.context, w, h);
    }

    pub fn swap_buffers(&self) {
        self.surface.swap_buffers(&self.context).unwrap();
    }

    fn load_fn(&self, symbol: &str) -> Result<*const c_void, ContextError> {
        let cstr = std::ffi::CString::new(symbol).unwrap();
        Ok(self.context.display().get_proc_address(&cstr))
    }

    pub fn glow(&self) -> Result<glow::Context, ContextError> {
        unsafe {
            Ok(glow::Context::from_loader_function_cstr(|s| {
                // Warn if the symbol is not found
                match self.load_fn(s.to_str().unwrap_or("")) {
                    Ok(addr) => addr,
                    Err(e) => {
                        // That's not a catastrophic, but we should know about it
                        warn!(
                            "Failed to load OpenGL symbol: {}: {}",
                            s.to_str().unwrap_or(""),
                            e
                        );
                        std::ptr::null()
                    }
                }
            }))
        }
    }
}
