use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;

pub enum SamplerMessage {
    Play { clip_id: usize, volume: f32 },
    StopAll,
}

/// Allows playing multiple audio clips in parallel,
/// without controlling the playback position.
pub struct SamplerSource {
    receiver: ControlReceiver<SamplerMessage>,
}

impl SamplerSource {
    pub fn new() -> (Self, Controller<SamplerMessage>) {
        let (controller, receiver) = new_control();
        let source = Self { receiver };
        (source, controller)
    }
}

impl EventDispatcher for SamplerSource {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                SamplerMessage::Play { clip_id, volume } => {
                    // Handle playing a clip with the given ID and volume
                    // For now, we just log it
                    log::info!("Playing clip {} at volume {}", clip_id, volume);
                }
                SamplerMessage::StopAll => {
                    // Handle stopping all clips
                    log::info!("Stopping all clips");
                }
            }
        }
    }
}

impl Generator for SamplerSource {
    fn generate(&mut self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        output.silence();
    }
}
