#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteName {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    name: NoteName,
    octave: u8,
}

impl Default for Note {
    fn default() -> Self {
        Note {
            name: NoteName::C,
            octave: 4,
        }
    }
}

impl Note {
    pub(crate) fn new(name: NoteName, octave: u8) -> Self {
        if octave < 0 || octave > 8 {
            panic!("Octave must be between 0 and 8");
        }
        Note { name, octave }
    }

    pub(crate) fn frequency(&self) -> f32 {
        let base_frequency = match self.name {
            NoteName::C => 261.63,
            NoteName::CSharp => 277.18,
            NoteName::D => 293.66,
            NoteName::DSharp => 311.13,
            NoteName::E => 329.63,
            NoteName::F => 349.23,
            NoteName::FSharp => 369.99,
            NoteName::G => 392.00,
            NoteName::GSharp => 415.30,
            NoteName::A => 440.00,
            NoteName::ASharp => 466.16,
            NoteName::B => 493.88,
        };
        base_frequency * 2f32.powi(self.octave as i32 - 4)
    }

    pub fn from_midi(midi_note: u8) -> Self {
        if midi_note < 21 || midi_note > 108 {
            panic!("MIDI note must be between 21 and 108");
        }
        let octave = (midi_note / 12) as u8;
        let note_index = midi_note % 12;
        let name = match note_index {
            0 => NoteName::C,
            1 => NoteName::CSharp,
            2 => NoteName::D,
            3 => NoteName::DSharp,
            4 => NoteName::E,
            5 => NoteName::F,
            6 => NoteName::FSharp,
            7 => NoteName::G,
            8 => NoteName::GSharp,
            9 => NoteName::A,
            10 => NoteName::ASharp,
            _ => NoteName::B,
        };
        Self { name, octave }
    }
}
