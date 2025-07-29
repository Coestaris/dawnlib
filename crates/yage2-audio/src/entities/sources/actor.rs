use crate::entities::{AudioEventTarget, AudioEventTargetId, AudioEventType, BlockInfo, Source};
use crate::resources::ClipResource;
use crate::sample::PlanarBlock;
use crate::{SamplesCount, BLOCK_SIZE};
use glam::Vec3;
use std::cmp::min;
use std::collections::HashMap;
use yage2_core::resources::Resource;

const MAX_ACTORS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorID(usize);

impl ActorID {
    const EMPTY: ActorID = ActorID(0);

    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Zero is reserved for the default target
        ActorID(id)
    }
}

#[derive(Debug, Clone)]
pub enum ActorsSourceEvent {
    AddActor {
        id: Option<ActorID>,
        pos: Vec3,
        gain: f32,
        clip: Resource,
    },
    RemoveActor(ActorID),
    ChangeActorPosition {
        id: ActorID,
        pos: Vec3,
    },
    ChangeActorGain {
        id: ActorID,
        gain: f32,
    },
    ChangeListenerPosition(Vec3),
}

struct Voice {
    id: ActorID,
    position: Vec3,
    gain: f32,
    playback_position: SamplesCount,
    clip: Option<Resource>,
}

impl Voice {
    pub fn new(id: ActorID, position: Vec3, gain: f32, clip: Resource) -> Self {
        Voice {
            id,
            position,
            gain,
            playback_position: 0,
            clip: Some(clip),
        }
    }
}

impl Default for Voice {
    fn default() -> Self {
        Voice {
            id: ActorID::EMPTY,
            position: Vec3::ZERO,
            playback_position: 0,
            gain: 1.0,
            clip: None,
        }
    }
}

pub struct ActorsSource {
    id: AudioEventTargetId,
    cached: bool,
    listener_position: Vec3,
    id_map: HashMap<ActorID, usize>,
    voices: [Voice; MAX_ACTORS],
    output: PlanarBlock<f32>,
}

fn dispatch_actors(ptr: *mut u8, event: &AudioEventType) {
    let actors: &mut ActorsSource = unsafe { &mut *(ptr as *mut ActorsSource) };
    actors.dispatch(event);
}

impl ActorsSource {
    pub fn new() -> Self {
        let mut voices = unsafe { std::mem::zeroed::<[Voice; MAX_ACTORS]>() };
        for voice in voices.iter_mut() {
            *voice = Voice::default();
        }
        ActorsSource {
            id: AudioEventTargetId::new(),
            cached: false,
            listener_position: Vec3::ZERO,
            id_map: HashMap::new(),
            voices,
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
            AudioEventType::Actors(ActorsSourceEvent::AddActor {
                id,
                pos,
                gain,
                clip,
            }) => {
                // Find free slot
                if let Some(index) = self.voices.iter_mut().position(|v| v.id == ActorID::EMPTY) {
                    let actor_id = id.unwrap_or_else(ActorID::new);
                    self.voices[index] = Voice::new(actor_id, *pos, *gain, clip.clone());
                    // TODO: What if the actor already exists?
                    self.id_map.insert(actor_id, index);
                    self.cached = false;
                } else {
                    log::warn!("No free voice slot available for new actor");
                }
            }
            AudioEventType::Actors(ActorsSourceEvent::RemoveActor(id)) => {
                if let Some(&index) = self.id_map.get(id) {
                    self.voices[index] = Voice::default();
                    self.id_map.remove(id);
                    self.cached = false;
                } else {
                    log::warn!("Attempted to remove non-existent actor: {:?}", id);
                }
            }
            AudioEventType::Actors(ActorsSourceEvent::ChangeActorPosition { id, pos }) => {
                if let Some(&index) = self.id_map.get(id) {
                    self.voices[index].position = *pos;
                    self.cached = false;
                } else {
                    log::warn!(
                        "Attempted to change position of non-existent actor: {:?}",
                        id
                    );
                }
            }
            AudioEventType::Actors(ActorsSourceEvent::ChangeActorGain { id, gain }) => {
                if let Some(&index) = self.id_map.get(id) {
                    self.voices[index].gain = *gain;
                    self.cached = false;
                } else {
                    log::warn!("Attempted to change gain of non-existent actor: {:?}", id);
                }
            }
            AudioEventType::Actors(ActorsSourceEvent::ChangeListenerPosition(pos)) => {
                self.listener_position = *pos;
                self.cached = false;
            }

            _ => {}
        }
    }

    fn frame_start(&mut self) {
        self.cached = false;
    }

    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        };

        self.output.silence();
        for actor in self.voices.iter_mut() {
            if actor.id == ActorID::EMPTY {
                continue; // Skip empty voices
            }

            if let Some(clip) = actor.clip.as_ref() {
                // TODO: Handle audio clip reading and gain application
                // TODO: Implement SIMD processing for performance

                // Copy audio data from the clip to the output
                let clip = clip.downcast_ref::<ClipResource>().unwrap();
                let to_copy = min(BLOCK_SIZE, clip.len - actor.playback_position);

                let mut block = PlanarBlock::default();
                block.copy_from_planar_vec(&clip.data, actor.playback_position, to_copy);
                self.output.addm(&block, actor.gain);
                actor.playback_position += to_copy;

                // Check if the playback is finished
                if actor.playback_position == clip.len {
                    actor.id = ActorID::EMPTY; // Reset voice if clip is finished
                    actor.clip = None; // Drop the clip
                }
            }
        }

        self.cached = true;
        &self.output
    }
}
