use crate::passes::events::PassEventTrait;
use glam::UVec2;

#[cfg(not(target_arch = "wasm32"))]
use crate::gl::context_glutin::Context;
#[cfg(target_arch = "wasm32")]
use crate::gl::context_webgl::Context;

pub(crate) trait RendererBackendTrait<E: PassEventTrait>
where
    Self: Sized,
{
    fn new(config: RendererConfig, context: Context) -> Result<Self, RendererBackendError>;

    fn before_frame(&mut self) -> Result<(), RendererBackendError>;
    fn after_frame(&mut self) -> Result<(), RendererBackendError>;
    fn resize(&self, size: UVec2) -> Result<(), RendererBackendError>;
}

mod backend_impl {
    pub type RendererBackend<E> = crate::gl::GLRenderer<E>;
    pub type RendererConfig = crate::gl::GLRendererConfig;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

pub use backend_impl::*;
