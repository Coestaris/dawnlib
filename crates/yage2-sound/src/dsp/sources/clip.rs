use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;

pub enum ClipMessage {
    Play,
    Pause,
    SetPosition(f32), // Position in seconds
}

/// Allows playing a single audio clip,
/// controlling the playback position.
pub struct ClipSource {
    receiver: ControlReceiver<ClipMessage>,
}

impl ClipSource {
    pub fn new() -> (Self, Controller<ClipMessage>) {
        let (controller, receiver) = new_control();
        let source = Self { receiver };
        (source, controller)
    }
}

impl EventDispatcher for ClipSource {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
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
