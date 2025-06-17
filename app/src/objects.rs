use log::info;
use yage2::engine::event::{Event, EventKind, KeyCode};
use yage2::engine::object::{Object, ObjectCtx, ObjectEvent};

pub(crate) struct EventListener;

impl Object for EventListener {
    fn events_mask(&self, _: &ObjectCtx) -> EventKind {
        EventKind::all()
    }

    fn on_event(&mut self, _: &ObjectCtx, event: &Event) -> Option<ObjectEvent> {
        match event {
            Event::KeyPress(KeyCode::Escape) => Some(ObjectEvent::QuitApplication),
            Event::KeyPress(KeyCode::BackSpace) => Some(ObjectEvent::Die),
            Event::Update(_) => None,
            _ => {
                info!("EventListener received event: {:?}", event);
                None
            }
        }
    }
}
