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
