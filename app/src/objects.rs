use log::info;
use yage2::engine::app_ctx::ApplicationCtx;
use yage2::engine::event::{InputEvent, InputEventKind};
use yage2::engine::object::Object;

pub(crate) struct EventListener;

impl Object for EventListener
{
    fn events_mask(&self, _: &ApplicationCtx) -> InputEventKind {
        InputEventKind::all()
    }

    fn on_event(&mut self, _: &ApplicationCtx, event: &InputEvent) {
        info!("EventListener received event: {:?}", event);
    }
}