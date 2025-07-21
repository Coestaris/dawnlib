use crate::entities::{Effect, Event, EventTarget, EventTargetId, Source};
use crate::sample::PlanarBlock;

const MAX_ACTORS: usize = 1024;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn zero() -> Self {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

pub struct ActorsSource {
    id: EventTargetId,
    cached: bool,
    listener_position: Vec3,
    positions: [Vec3; MAX_ACTORS],
    gains: [f32; MAX_ACTORS],
    output: PlanarBlock<f32>,
}

fn dispatch_actors(ptr: *mut u8, event: &Event) {
    let actors: &mut ActorsSource = unsafe { &mut *(ptr as *mut ActorsSource) };
    actors.dispatch(event);
}

impl ActorsSource {
    pub fn new() -> Self {
        ActorsSource {
            id: EventTargetId::new(),
            cached: false,
            listener_position: Vec3::zero(),
            positions: [Vec3::zero(); MAX_ACTORS],
            gains: [0.0; MAX_ACTORS],
            output: Default::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget {
            id: self.id,
            dispatcher: dispatch_actors,
            ptr: self as *const _ as *mut u8,
        }
    }
}

impl Source for ActorsSource {
    fn get_targets(&self) -> Vec<EventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::AddActor { id, gain } => {
                // TODO: Implement logic to add an actor
            }
            Event::RemoveActors(id) => {
                // TODO: Implement logic to remove an actor
            }
            Event::ChangeListenerPosition {} => {}

            _ => {
                // Handle other events if needed
            }
        }
    }

    fn frame_start(&mut self) {
        self.cached = false;
    }

    fn render(&mut self) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        };

        // Reset output block
        self.output.silence();
        // TODO: Implement actual audio processing logic here
        self.cached = true;
        &self.output
    }
}
