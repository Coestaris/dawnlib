use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum IRNoteEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    Idle { ms: f32 },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IRNotes {
    pub events: Vec<IRNoteEvent>,
}

impl IRNotes {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRNotes>();
        sum += self.events.capacity() * size_of::<IRNoteEvent>();
        sum
    }
}
