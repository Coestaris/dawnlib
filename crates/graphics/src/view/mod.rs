#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
// TODO: Support for Wayland
mod x11;

use crate::input::InputEvent;
use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

#[cfg(target_os = "macos")]
pub mod view_impl {
    use crate::view::darwin;

    pub type PlatformSpecificViewConfig = darwin::PlatformSpecificViewConfig;
    pub type ViewError = darwin::ViewError;
    pub(crate) type View = darwin::View;

    pub use crate::view::darwin::ViewHandle;
}

#[cfg(target_os = "linux")]
pub mod view_impl {
    use crate::view::x11;

    pub type PlatformSpecificViewConfig = x11::PlatformSpecificViewConfig;
    pub type ViewError = x11::ViewError;
    pub(crate) type View = x11::View;

    pub use crate::view::x11::ViewHandle;
}

#[cfg(target_os = "windows")]
pub mod view_impl {
    use crate::view::windows;

    pub type PlatformSpecificViewConfig = windows::PlatformSpecificViewConfig;
    pub type ViewError = windows::ViewError;
    pub(crate) type View = windows::View;

    pub use crate::view::windows::ViewHandle;
}

pub use view_impl::*;

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
    fn open(cfg: ViewConfig, events_sender: Arc<ArrayQueue<InputEvent>>) -> Result<Self, ViewError>
    where
        Self: Sized;

    fn get_handle(&self) -> ViewHandle;

    fn tick(&mut self) -> TickResult;

    fn set_size(&self, width: usize, height: usize);
    fn set_title(&self, title: &str);
}
