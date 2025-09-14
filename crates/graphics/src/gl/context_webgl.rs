use glam::UVec2;
use thiserror::Error;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

pub struct Context {}

#[derive(Debug, Error)]
pub enum ContextError {

}

impl Context {
    pub fn create_contextual_window(
        attributes: WindowAttributes,
        event_loop: &ActiveEventLoop,
    ) -> Result<(Window, Context), ContextError> {
        todo!()
    }

    pub fn resize(&self, size: UVec2) {
        todo!()
    }

    pub fn swap_buffers(&self) {
        todo!()
    }

    pub fn glow(&self) -> Result<glow::Context, ContextError> {
        todo!()
    }
}
