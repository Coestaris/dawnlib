use crate::control::{new_control, ControlReceiver, Controller};
use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::resources::ClipResource;
use crate::sample::PlanarBlock;
use crate::BLOCK_SIZE;
use yage2_core::resources::Resource;

pub enum SamplerMessage {
    Play {
        clip: Resource,
        volume: f32,
        pan: f32,
    },
    StopAll,
}

struct Player {
    clip: Resource,
    volume: f32,
    position: usize, // Current playback position in samples
    pan: f32,
}

/// Allows playing multiple audio clips in parallel,
/// without controlling the playback position.
pub struct SamplerSource {
    receiver: ControlReceiver<SamplerMessage>,
    players: Vec<Player>,
}

impl SamplerSource {
    pub fn new() -> (Self, Controller<SamplerMessage>) {
        let (controller, receiver) = new_control();
        let source = Self {
            receiver,
            players: vec![],
        };
        (source, controller)
    }
}

impl EventDispatcher for SamplerSource {
    fn dispatch_events(&mut self) {
        while let Some(message) = self.receiver.receive() {
            match message {
                SamplerMessage::Play { clip, volume, pan } => {
                    self.players.push(Player {
                        clip,
                        volume,
                        pan,
                        position: 0,
                    });
                }
                SamplerMessage::StopAll => {
                    log::info!("Stopping all clips");
                    self.players.clear();
                }
            }
        }
    }
}

impl Generator for SamplerSource {
    fn generate(&mut self, output: &mut PlanarBlock<f32>, _: &BlockInfo) {
        let mut some_deleted = false;

        // Iterate over all players and generate audio samples
        for player in &mut self.players {
            let mut player_block = PlanarBlock::default();

            let clip = player.clip.downcast_ref::<ClipResource>().unwrap();
            // Generate audio data from the clip resource
            let to_copy = (clip.len as usize - player.position).min(BLOCK_SIZE);

            // Copy audio data from the clip resource to the output block
            player_block.copy_from_planar_vec(&clip.data, player.position, to_copy);

            // Apply volume and pan
            unsafe {
                player_block.pan_gain_simd(player.pan, player.volume);
            }
            
            // Mix the player's output into the main output
            unsafe {
                output.mix_simd(&player_block);
            }

            // Update the player's position
            player.position += to_copy;
            if player.position >= clip.len as usize {
                // Reset position if we reached the end of the clip
                player.position = 0;
                some_deleted = true;
            }
        }

        // Remove finished players if any
        if some_deleted {
            self.players.retain(|player| player.position > 0);
        }
    }
}
