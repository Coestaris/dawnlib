use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::resources::ClipResource;
use crate::sample::PlanarBlock;
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
    pub fn new() -> (Self, Controller<ClipMessage>) {
        let (controller, receiver) = new_control();
        let source = Self {
            receiver,
            playing_clip: None,
            position: 0, // Start at the beginning of the clip
        };
        (source, controller)
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
    fn generate(&mut self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        if let Some(res) = &self.playing_clip {
            let clip = res.downcast_ref::<ClipResource>().unwrap();

            // Generate audio data from the clip resource
            // assume for now that sample rate is same as output sample rate
            // assume that sample is monophonic
            let to_copy = (clip.len as usize - self.position).min(BLOCK_SIZE);

            // Copy audio data from the clip resource to the output block
            for i in 0..to_copy {
                // TODO: Implement some kind of batch processing
                output.samples[0][i] = clip.data[self.position + i];
                output.samples[1][i] = clip.data[self.position + i];
            }
            
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
