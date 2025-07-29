use crate::event::Event;
use crate::view::{TickResult, ViewHandle, ViewTrait};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct PlatformSpecificViewConfig {}

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
