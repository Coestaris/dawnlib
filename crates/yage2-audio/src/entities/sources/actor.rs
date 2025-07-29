use crate::entities::{BlockInfo, AudioEventType, AudioEventTarget, AudioEventTargetId, Source};
use crate::sample::PlanarBlock;
use glam::Vec3;

const MAX_ACTORS: usize = 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum ActorsSourceEvent {
    AddActor { pos: Vec3, id: usize, gain: f32 },
    RemoveActors(usize),
    ChangeListenerPosition(Vec3),
}

pub struct ActorsSource {
    id: AudioEventTargetId,
    cached: bool,
    listener_position: Vec3,
    positions: [Vec3; MAX_ACTORS],
    gains: [f32; MAX_ACTORS],
    output: PlanarBlock<f32>,
}

fn dispatch_actors(ptr: *mut u8, event: &AudioEventType) {
    let actors: &mut ActorsSource = unsafe { &mut *(ptr as *mut ActorsSource) };
    actors.dispatch(event);
}

impl ActorsSource {
    pub fn new() -> Self {
        ActorsSource {
            id: AudioEventTargetId::new(),
            cached: false,
            listener_position: Vec3::ZERO,
            positions: [Vec3::ZERO; MAX_ACTORS],
            gains: [0.0; MAX_ACTORS],
            output: Default::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_actors, self.id, self)
    }
}

impl Source for ActorsSource {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::Actors(ActorsSourceEvent::AddActor { pos, id, gain }) => {
                // TODO: Implement logic to add an actor
            }
            AudioEventType::Actors(ActorsSourceEvent::RemoveActors(id)) => {
                // TODO: Implement logic to remove an actor
            }
            AudioEventType::Actors(ActorsSourceEvent::ChangeListenerPosition(pos)) => {}

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
