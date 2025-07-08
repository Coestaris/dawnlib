#[cfg(target_os = "macos")]
mod darwin;

use crate::event::Event;
use crate::vulkan::objects::surface::Surface;
use std::sync::mpsc::Sender;

#[cfg(target_os = "macos")]
pub mod view_impl {
    use crate::view::darwin;

    pub type PlatformSpecificViewConfig = darwin::ViewConfig;
    pub type ViewError = darwin::ViewError;
    pub(crate) type View = darwin::View;
}

pub use view_impl::*;
use crate::vulkan::GraphicsError;

#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub platform_specific: PlatformSpecificViewConfig,
    pub title: String,
    pub width: usize,
    pub height: usize,
}

pub(crate) enum TickResult {
    Continue,
    Closed,
    Failed(ViewError),
}

pub(crate) trait ViewTrait {
    fn open(cfg: ViewConfig, events_sender: Sender<Event>) -> Result<Self, ViewError>
    where
        Self: Sized;

    fn create_surface(
        &self,
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<Surface, GraphicsError>;

    fn tick(&mut self) -> TickResult;

    fn set_size(&self, width: usize, height: usize);
    fn set_title(&self, title: &str);
}
