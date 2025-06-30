use log::info;
use yage2_app::create_object;
use yage2_app::engine::event::{Event, EventMask, KeyCode};
use yage2_app::engine::object::{DispatchAction, Object, ObjectCtx, Renderable};

pub(crate) struct EventListener;

impl Object for EventListener {
    fn event_mask(&self) -> EventMask {
        EventMask::all()
    }

    fn dispatch(&mut self, _: &ObjectCtx, event: &Event) -> DispatchAction {
        match event {
            Event::Create => {
                info!("EventListener created");
                DispatchAction::Empty
            }
            Event::KeyPress(KeyCode::Escape) => DispatchAction::QuitApplication,
            Event::KeyPress(KeyCode::BackSpace) => DispatchAction::Die,
            Event::KeyPress(KeyCode::Latin('r' | 'R')) => DispatchAction::SpawnObject(
                create_object!(SimpleObject::new(Point::new(22.0, 22.0))),
            ),
            Event::Update(_) => DispatchAction::Empty,
            _ => {
                // info!("EventListener received event: {:?}", event);
                DispatchAction::Empty
            }
        }
    }
}

pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Point { x, y }
    }

    fn distance_to(&self, other: &Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

pub(crate) struct SimpleObject {
    point: Point,
}

impl SimpleObject {
    pub(crate) fn new(point: Point) -> Self {
        SimpleObject { point }
    }
}

impl Object for SimpleObject {
    fn event_mask(&self) -> EventMask {
        EventMask::MOUSE_BUTTON_RELEASE
            | EventMask::CREATE
            | EventMask::KEY_RELEASE
            | EventMask::CREATE
    }

    fn dispatch(&mut self, ctx: &ObjectCtx, event: &Event) -> DispatchAction {
        match event {
            Event::Create => {
                info!("SimpleObject created");
                DispatchAction::Empty
            }
            Event::KeyRelease(KeyCode::Latin('s' | 'S')) => {
                DispatchAction::UpdateRenderable(Renderable {
                    sample_data: 4.0,
                    sample_data2: 3,
                })
            }

            Event::KeyRelease(KeyCode::Latin('h' | 'H')) => DispatchAction::DeleteRenderable,

            Event::MouseButtonRelease(_) => {
                let (x, y) = ctx.input_manager.mouse_position();
                let mouse_point = Point::new(x, y);
                let distance = self.point.distance_to(&mouse_point);
                info!(
                    "Mouse released at ({}, {}), distance to point ({}, {}): {}",
                    x, y, self.point.x, self.point.y, distance
                );

                DispatchAction::Empty
            }
            _ => DispatchAction::Empty,
        }
    }
}
