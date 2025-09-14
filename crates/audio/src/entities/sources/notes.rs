use crate::assets::NotesAsset;
use crate::entities::bus::Bus;
use crate::entities::effects::bypass::BypassEffect;
use crate::entities::effects::fir::FirFilterEffect;
use crate::entities::effects::multiplexer::Multiplexer2Effect;
use crate::entities::effects::soft_clip::SoftClipEffect;
use crate::entities::events::{AudioEvent, AudioEventTargetId, AudioEventType};
use crate::entities::sources::multiplexer::MultiplexerSource;
use crate::entities::sources::waveform::{WaveformSource, WaveformSourceEvent, WaveformType};
use crate::player::Player;
use crate::SampleRate;
use dawn_assets::ir::notes::IRNoteEvent;
use dawn_assets::TypedAsset;
use log::warn;
use std::thread::sleep;
use web_time::Duration;
use tinyrand::Rand;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Voice {
    target: AudioEventTargetId,
    note: Note,
    playing: bool,
}

pub struct NotesPlayer<const VOICES_COUNT: usize> {
    voices: [Voice; VOICES_COUNT],
    // Currently processing event index
    index: usize,
    asset: TypedAsset<NotesAsset>,
}

impl<const VOICES_COUNT: usize> NotesPlayer<VOICES_COUNT> {
    pub fn new<'a>(
        midi: TypedAsset<NotesAsset>,
        sample_rate: SampleRate,
    ) -> (
        NotesPlayer<VOICES_COUNT>,
        Bus<
            Multiplexer2Effect<FirFilterEffect<32>, SoftClipEffect>,
            MultiplexerSource<Bus<BypassEffect, WaveformSource>, VOICES_COUNT>,
        >,
    ) {
        let mut voices: [Voice; VOICES_COUNT] = unsafe { std::mem::zeroed() };
        let mut busses: [_; VOICES_COUNT] = unsafe { std::mem::zeroed() };
        let mut rng = tinyrand::Wyrand::default();
        for i in 0..VOICES_COUNT {
            let source = WaveformSource::new(None);
            let bus_effect = BypassEffect::new();

            voices[i] = Voice {
                target: source.get_id(),
                note: Note::default(),
                playing: false,
            };
            busses[i] = Bus::new(
                bus_effect,
                source,
                Some(1.0 / VOICES_COUNT as f32),
                Some(rng.next_u32() as f32 / u32::MAX as f32 * 0.8 - 0.4),
            );
        }
        let source = MultiplexerSource::new(busses);

        let filter = FirFilterEffect::new_from_design(5000.0, sample_rate as f32);
        let clipper = SoftClipEffect::new(0.5, 2.0);
        let master_effect = Multiplexer2Effect::new(filter, clipper);

        (
            NotesPlayer {
                voices,
                asset: midi,
                index: 0,
            },
            Bus::new(master_effect, source, None, None),
        )
    }

    fn set_freq(&self, player: &Player, target: AudioEventTargetId, freq: f32) {
        let event = AudioEvent::new(
            target,
            AudioEventType::Waveform(WaveformSourceEvent::SetWaveformType(
                WaveformType::Sawtooth(freq),
            )),
        );
        player.push_event(&event);
    }

    fn mute(&self, player: &Player, target: AudioEventTargetId) {
        let event = AudioEvent::new(
            target,
            AudioEventType::Waveform(WaveformSourceEvent::SetWaveformType(WaveformType::Disabled)),
        );
        player.push_event(&event);
    }

    fn play_note(&mut self, player: &Player, midi_note: u8) {
        // Find free voice
        match self.voices.iter().position(|v| !v.playing) {
            Some(index) => {
                // Occupy the voice
                let note = Note::from_midi(midi_note);
                self.voices[index].note = note;
                self.voices[index].playing = true;

                // Set frequency for the voice
                let freq = note.frequency();
                self.set_freq(player, self.voices[index].target, freq);
            }
            None => warn!("No free voice found"),
        }
    }

    fn stop_note(&mut self, player: &Player, midi_note: u8) {
        let note = Note::from_midi(midi_note);

        // Find the voice playing the note
        if let Some(index) = self.voices.iter().position(|v| v.playing && v.note == note) {
            // Mute the voice
            self.mute(player, self.voices[index].target);
            // Mark the voice as not playing
            self.voices[index].playing = false;
        }
    }

    pub fn play(&mut self, player: &Player) {
        // Using indirection to not borrow self mutably
        let r = self.asset.clone();
        let r = r.cast();
        let events = &r.0.events;

        while self.index < events.len() {
            match &events[self.index] {
                IRNoteEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    self.play_note(player, *note);
                }
                IRNoteEvent::NoteOff { channel, note } => {
                    self.stop_note(player, *note);
                }
                IRNoteEvent::Idle { ms } => {
                    sleep(Duration::from_micros((*ms * 1000.0) as u64));
                }
                _ => {}
            }
            self.index += 1;
        }
    }
}
