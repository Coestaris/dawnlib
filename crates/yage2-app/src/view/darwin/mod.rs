use crate::event::Event;
use crate::view::{TickResult, ViewTrait};
use crate::vulkan::objects::surface::Surface;
use crate::vulkan::GraphicsError;
use ash::{Entry, Instance};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct ViewConfig {}

#[derive(Debug)]
pub enum ViewError {}

pub(crate) struct View {}

impl ViewTrait for View {
    fn open(cfg: crate::view::ViewConfig, events_sender: Sender<Event>) -> Result<Self, ViewError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn create_surface(&self, entry: &Entry, instance: &Instance) -> Result<Surface, GraphicsError> {
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
