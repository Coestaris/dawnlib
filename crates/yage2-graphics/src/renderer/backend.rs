pub(crate) trait RendererBackendTrait<E>
where
    E: Copy + 'static,
    Self: Sized,
{
    fn new(
        config: RendererBackendConfig,
        view_handle: ViewHandle,
    ) -> Result<Self, RendererBackendError>;

    fn before_frame(&mut self) -> Result<(), RendererBackendError>;
    fn after_frame(&mut self) -> Result<(), RendererBackendError>;
}

#[cfg(feature = "gl")]
mod backend_impl {
    pub type RendererBackend<E> = crate::gl::GLRenderer<E>;
    pub type RendererBackendConfig = crate::gl::GLRendererConfig;
    pub type RendererBackendError = crate::gl::GLRendererError;
}

pub use backend_impl::*;
