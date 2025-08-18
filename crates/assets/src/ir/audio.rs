use serde::{Deserialize, Serialize};

/// Internal representation of audio data
/// Always storing samples in the F32 sample format
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IRAudio {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u8,
    pub length: usize, // In samples
}

impl Default for IRAudio {
    fn default() -> Self {
        IRAudio {
            data: vec![],
            sample_rate: 44100,
            channels: 2,
            length: 0,
        }
    }
}