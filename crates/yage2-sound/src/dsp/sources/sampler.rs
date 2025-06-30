use crate::dsp::{BlockInfo, Control, Generator};
use crate::sample::PlanarBlock;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum SamplerMessage {
    Play { clip_id: usize, volume: f32 },
    StopAll,
}

/// Allows playing multiple audio clips in parallel,
/// without controlling the playback position.
pub struct SamplerSource {
    receiver: Receiver<SamplerMessage>,
}

impl SamplerSource {
    pub fn new() -> (Self, Sender<SamplerMessage>) {
        let (sender, receiver) = channel();
        let source = Self { receiver };
        (source, sender)
    }
}

impl Control for SamplerSource {
    fn process_events(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
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
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        output.silence();
    }
}
