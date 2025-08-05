mod bindings;

use crate::renderable::Renderable;
use crate::renderer::{
    RendererBackendConfig, RendererBackendError, RendererBackendTrait, RendererTickResult,
};
use crate::view::{ViewHandle, ViewHandleTrait};
use std::fmt::{Display, Formatter};

pub struct GLRenderer {
    view_handle: ViewHandle,
}
pub struct GLRendererConfig {
    pub fps: usize,
    pub vsync: bool,
}

#[derive(Debug, Clone)]
pub enum GLRendererError {}

impl Display for GLRendererError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred in the graphics module")
    }
}

impl std::error::Error for GLRendererError {}

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
        Ok(GLRenderer { view_handle })
    }

    fn tick(
        &mut self,
        renderables: &[Renderable],
    ) -> Result<RendererTickResult, RendererBackendError> {
        self.view_handle.swap_buffers().unwrap();

        unsafe {
            bindings::ClearColor(0.0, 0.2, 0.0, 1.0);
            bindings::Clear(bindings::COLOR_BUFFER_BIT);
        }

        Ok(RendererTickResult {
            draw_calls: 0,
            drawn_primitives: 0,
        })
    }
}
