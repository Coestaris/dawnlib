#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "linux")]
// TODO: Support for Wayland
mod x11;

use crate::event::Event;
use std::sync::mpsc::Sender;
use ash::vk;

#[cfg(target_os = "macos")]
pub mod view_impl {
    use crate::view::darwin;

    pub type PlatformSpecificViewConfig = darwin::PlatformSpecificViewConfig;
    pub type ViewError = darwin::ViewError;
    pub(crate) type View = darwin::View;
}

#[cfg(target_os = "linux")]
pub mod view_impl {
    use crate::view::x11;

    pub type PlatformSpecificViewConfig = x11::PlatformSpecificViewConfig;
    pub type ViewError = x11::ViewError;
    pub(crate) type View = x11::View;
}

pub use view_impl::*;

pub(crate) enum ViewHandle {
    Darwin {},
    Windows {
        hinstance: vk::HINSTANCE,
        hwnd: vk::HWND,
    },
    X11 {
        display: *mut vk::Display,
        window: vk::Window,
    },
    Wayland,
}

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

    fn get_handle(&self) -> ViewHandle;

    fn tick(&mut self) -> TickResult;

    fn set_size(&self, width: usize, height: usize);
    fn set_title(&self, title: &str);
}
