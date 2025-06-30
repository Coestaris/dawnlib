use crate::dsp::{BlockInfo, Control, Generator};
use crate::sample::PlanarBlock;
use std::sync::mpsc::{Receiver, Sender};

pub enum ClipMessage {
    Play,
    Pause,
    SetPosition(f32), // Position in seconds
}

/// Allows playing a single audio clip,
/// controlling the playback position.
pub struct ClipSource {
    receiver: Receiver<ClipMessage>,
}

impl ClipSource {
    pub fn new() -> (Self, Sender<ClipMessage>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let source = Self { receiver };
        (source, sender)
    }
}

impl Control for ClipSource {
    fn process_events(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                ClipMessage::Play => {
                    // Handle play logic
                    log::info!("ClipSource: Play");
                }
                ClipMessage::Pause => {
                    // Handle pause logic
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
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        output.silence();
    }
}
