#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
// TODO: Support for Wayland
mod x11;

use crate::input::InputEvent;
use crossbeam_channel::Sender;
use dawn_util::rendezvous::Rendezvous;

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

#[derive(Clone)]
pub struct ViewSynchronization {
    pub before_frame: Rendezvous,
    pub after_frame: Rendezvous,
}

#[derive(Clone, Debug)]
pub enum ViewCursor {
    Default,
    Hidden,

    Crosshair,
    Hand,
    Arrow,
    Move,
    Text,
    Wait,
    Help,
    NotAllowed,
}

#[derive(Clone, Debug)]
pub enum ViewGeometry {
    Normal(u32, u32),
    BorderlessFullscreen,
    Fullscreen,
}

#[derive(Clone)]
pub struct ViewConfig {
    /// Platform-specific configuration
    pub platform_specific: PlatformSpecificViewConfig,

    /// Allows to enable additional synchronization between threads.
    /// For example, to synchronize rendering and logic threads.
    pub synchronization: Option<ViewSynchronization>,

    /// Title of the window
    pub title: String,
    /// Initial geometry of the window
    pub geometry: ViewGeometry,
    /// Initial cursor style
    pub cursor: ViewCursor,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum TickResult {
    Continue,
    Closed,
    Failed(ViewError),
}

pub(crate) trait ViewTrait {
    fn open(cfg: ViewConfig, events_sender: Sender<InputEvent>) -> Result<Self, ViewError>
    where
        Self: Sized;

    fn get_handle(&self) -> ViewHandle;

    fn tick(&mut self) -> TickResult;

    fn set_geometry(&mut self, geometry: ViewGeometry) -> Result<(), ViewError>;
    fn set_title(&mut self, title: &str) -> Result<(), ViewError>;
    fn set_cursor(&mut self, cursor: ViewCursor) -> Result<(), ViewError>;
}
