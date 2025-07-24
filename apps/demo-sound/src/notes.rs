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

pub struct Note {
    name: NoteName,
    octave: u8,
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
}
