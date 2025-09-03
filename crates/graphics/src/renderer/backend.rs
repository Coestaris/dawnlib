use crate::passes::events::PassEventTrait;
use winit::raw_window_handle::RawWindowHandle;

pub(crate) trait RendererBackendTrait<E: PassEventTrait>
where
    Self: Sized,
{
    fn new(
        config: RendererConfig,
        handle: RawWindowHandle,
    ) -> Result<Self, RendererBackendError>;

    fn before_frame(&mut self) -> Result<(), RendererBackendError>;
    fn after_frame(&mut self) -> Result<(), RendererBackendError>;
}

#[cfg(feature = "gl")]
mod backend_impl {
    pub type RendererBackend<E> = crate::gl::GLRenderer<E>;
    pub type RendererConfig = crate::gl::GLRendererConfig;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

pub use backend_impl::*;
