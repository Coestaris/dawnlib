use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Internal representation of audio data
/// Always storing samples in the F32 sample format
#[derive(Serialize, Deserialize, Clone)]
pub struct IRAudio {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u8,
    pub length: usize, // In samples
}

impl Debug for IRAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRAudio")
            .field("data_length", &self.data.len())
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("length", &self.length)
            .finish()
    }
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
