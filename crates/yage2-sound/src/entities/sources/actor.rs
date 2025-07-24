use crate::entities::{BlockInfo, Event, EventTarget, EventTargetId, Source};
use crate::sample::PlanarBlock;
use yage2_core::vec3::Vec3;

const MAX_ACTORS: usize = 1024;

pub enum ActorsSourceEvent {
    AddActor { pos: Vec3, id: usize, gain: f32 },
    RemoveActors(usize),
    ChangeListenerPosition(Vec3),
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
        EventTarget::new(dispatch_actors, self.id, self)
    }
}

impl Source for ActorsSource {
    fn get_targets(&self) -> Vec<EventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Actors(ActorsSourceEvent::AddActor { pos, id, gain }) => {
                // TODO: Implement logic to add an actor
            }
            Event::Actors(ActorsSourceEvent::RemoveActors(id)) => {
                // TODO: Implement logic to remove an actor
            }
            Event::Actors(ActorsSourceEvent::ChangeListenerPosition(pos)) => {}

            _ => {
                // Handle other events if needed
            }
        }
    }

    fn frame_start(&mut self) {
        self.cached = false;
    }

    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
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
