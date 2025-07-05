use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Generator, SourceType};
use crate::resources::ClipResource;
use crate::sample::{PlanarBlock, LEFT_CHANNEL, RIGHT_CHANNEL};
use crate::BLOCK_SIZE;
use yage2_core::resources::Resource;

pub enum ClipMessage {
    Play(Resource),
    Pause,
    SetPosition(f32), // Position in seconds
}

/// Allows playing a single audio clip,
/// controlling the playback position.
pub struct ClipSource {
    receiver: ControlReceiver<ClipMessage>,
    playing_clip: Option<Resource>,
    position: usize, // Current playback position in samples
}

impl ClipSource {
    pub fn new() -> (SourceType, Controller<ClipMessage>) {
        let (controller, receiver) = new_control();
        let source = Self {
            receiver,
            playing_clip: None,
            position: 0, // Start at the beginning of the clip
        };
        (SourceType::Clip(source), controller)
    }
}

impl EventDispatcher for ClipSource {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                ClipMessage::Play(clip) => {
                    self.playing_clip = Some(clip);
                    log::info!("ClipSource: Play");
                }
                ClipMessage::Pause => {
                    self.playing_clip = None;
                    log::info!("ClipSource: Pause");
                }
                ClipMessage::SetPosition(position) => {
                    // Handle setting playback position
                    log::info!("ClipSource: Set position to {}", position);
                }
            }
        }
    }
}

impl Generator for ClipSource {
    fn generate(&mut self, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        if let Some(res) = &self.playing_clip {
            let clip = res.downcast_ref::<ClipResource>().unwrap();

            // Generate audio data from the clip resource
            let to_copy = (clip.len as usize - self.position).min(BLOCK_SIZE);

            // Copy audio data from the clip resource to the output block
            output.copy_from_planar_vec(&clip.data, self.position, to_copy);

            self.position += to_copy;
            if self.position >= clip.len as usize {
                // Reset position if we reached the end of the clip
                self.position = 0;
                log::info!("ClipSource: Reached end of clip, resetting position");
            }
        } else {
            output.silence();
        }
    }
}
