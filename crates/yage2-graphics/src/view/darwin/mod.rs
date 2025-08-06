use crate::input::InputEvent;
use crate::view::{TickResult, ViewConfig, ViewHandleTrait, ViewTrait};
use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PlatformSpecificViewConfig {}

#[derive(Debug)]
pub enum ViewError {}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ViewError")
    }
}

impl std::error::Error for ViewError {}

pub(crate) struct View {}

impl ViewTrait for View {
    fn open(
        cfg: ViewConfig,
        events_sender: Arc<ArrayQueue<InputEvent>>,
    ) -> Result<Self, crate::view::ViewError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn get_handle(&self) -> ViewHandle {
        todo!()
    }

    fn tick(&mut self) -> TickResult {
        todo!()
    }

    fn set_size(&self, width: usize, height: usize) {
        todo!()
    }

    fn set_title(&self, title: &str) {
        todo!()
    }
}

pub struct ViewHandle {}

#[cfg(feature = "gl")]
impl ViewHandleTrait for ViewHandle {
    fn create_context(&mut self, fps: usize, vsync: bool) -> Result<(), ViewError> {
        todo!()
    }

    fn get_proc_addr(&self, symbol: &str) -> Result<*const std::ffi::c_void, ViewError> {
        todo!()
    }

    fn swap_buffers(&self) -> Result<(), ViewError> {
        todo!()
    }
}
