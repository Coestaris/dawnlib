use crate::engine::input::InputEvent;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub trait Window<PlatformError, Graphics> {
    fn tick(&mut self) -> Result<bool, PlatformError>;
    fn kill(&mut self) -> Result<(), PlatformError>;
    fn get_graphics(&mut self) -> &mut Graphics;
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub trait WindowFactory<Win, PlatformError, Graphics>: Send + Sync {
    fn new(config: WindowConfig) -> Result<Self, PlatformError>
    where
        Self: Sized;

    fn create_window(&self, events_sender: Sender<InputEvent>) -> Result<Win, PlatformError>
    where
        Win: Window<PlatformError, Graphics> + Sized;
}
