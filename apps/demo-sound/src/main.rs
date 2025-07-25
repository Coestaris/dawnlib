mod notes;

use crate::notes::{Note, NoteName};
use common::logging::CommonLogger;
use common::resources::YARCResourceManagerIO;
use log::{debug, info, warn};
use midly::MidiMessage;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use yage2_core::resources::{
    Resource, ResourceFactory, ResourceHeader, ResourceManager, ResourceManagerConfig, ResourceType,
};
use yage2_core::threads::{scoped, ThreadManagerConfig, ThreadPriority};
use yage2_sound::backend::PlayerBackendConfig;
use yage2_sound::entities::bus::Bus;
use yage2_sound::entities::effects::bypass::BypassEffect;
use yage2_sound::entities::events::{Event, EventBox, EventTargetId};
use yage2_sound::entities::sinks::InterleavedSink;
use yage2_sound::entities::sources::multiplexer::MultiplexerSource;
use yage2_sound::entities::sources::waveform::{WaveformSource, WaveformSourceEvent, WaveformType};
use yage2_sound::player::{Player, PlayerConfig, ProfileFrame};
use yage2_sound::resources::{FLACResourceFactory, OGGResourceFactory, WAVResourceFactory};

fn profile_player(frame: &ProfileFrame) {
    // Calculate the time in milliseconds, the renderer thread
    // is maximally allowed to take to fill the device buffer.
    let allowed_time = (1000.0 / frame.sample_rate as f32) * frame.block_size as f32;

    // When no events are processed, we cannot calculate the load
    // (since the thread is not running).
    // Assume that the events thread has the same maximum allowed time
    // as the renderer thread.
    let events_load_precent = if frame.events_tps_av == 0.0 {
        0.0
    } else {
        frame.events_av / allowed_time * 100.0
    };

    info!(
        "T: {:.0}. Render: {:.1}ms ({:.1}%). Ev {:.1}ms ({:.1}%) ({:.0})",
        frame.render_tps_av,
        frame.render_av,
        frame.render_av / allowed_time * 100.0,
        frame.events_av,
        events_load_precent,
        frame.events_tps_av,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Voice {
    target: EventTargetId,
    note: Note,
    playing: bool,
}

struct MidiPlayer<const VOICES_COUNT: usize> {
    voices: [Voice; VOICES_COUNT],
    // Currently processing event index
    index: usize,
    midi: Resource,
}

impl<const VOICES_COUNT: usize> MidiPlayer<VOICES_COUNT> {
    fn new<'a>(
        midi: Resource,
    ) -> (
        MidiPlayer<VOICES_COUNT>,
        Bus<'a, BypassEffect, MultiplexerSource<'a, WaveformSource, VOICES_COUNT>>,
    ) {
        fn leak<T>(value: T) -> &'static T {
            Box::leak(Box::new(value))
        }
        let sources: [&WaveformSource; VOICES_COUNT] =
            std::array::from_fn(|_| leak(WaveformSource::new(None, None, None)));
        let voices: [Voice; VOICES_COUNT] = sources
            .iter()
            .map(|source| Voice {
                target: source.get_id(),
                note: Note::default(),
                playing: false,
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let gain: f32 = 3.0 / VOICES_COUNT as f32;
        let mixes: [f32; VOICES_COUNT] = [gain; VOICES_COUNT];
        let multiplexer = leak(MultiplexerSource::new(sources, mixes));
        let effect = leak(BypassEffect::new());

        (
            MidiPlayer {
                voices,
                midi,
                index: 0,
            },
            Bus::new(effect, multiplexer),
        )
    }

    fn set_freq(&self, player: &Player, target: EventTargetId, freq: f32) {
        let event = EventBox::new(
            target,
            Event::Waveform(WaveformSourceEvent::SetFrequency(freq)),
        );
        player.push_event(&event);
        let event = EventBox::new(
            target,
            Event::Waveform(WaveformSourceEvent::SetWaveformType(WaveformType::Sawtooth)),
        );
        player.push_event(&event);
    }

    fn mute(&self, player: &Player, target: EventTargetId) {
        let event = EventBox::new(
            target,
            Event::Waveform(WaveformSourceEvent::SetWaveformType(WaveformType::Disabled)),
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

    fn play(&mut self, player: &Player) {
        // Using indirection to not borrow self mutably
        let r = self.midi.clone();
        let r = r.downcast_ref::<MIDIResource>().unwrap();
        let events = &r.events;

        while self.index < events.len() {
            match &events[self.index] {
                MIDIEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    self.play_note(player, *note);
                }
                MIDIEvent::NoteOff { channel, note } => {
                    self.stop_note(player, *note);
                }
                MIDIEvent::Idle { ms } => {
                    sleep(Duration::from_micros((*ms * 1000.0) as u64));
                }
                _ => {}
            }
            self.index += 1;
        }
    }
}

enum MIDIEvent {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    PitchBend {
        channel: u8,
        value: i16,
    },
    Idle {
        ms: f32,
    },
}

struct MIDIResource {
    events: Vec<MIDIEvent>,
}

struct MIDIResourceFactory {}

impl MIDIResourceFactory {
    fn new() -> Self {
        MIDIResourceFactory {}
    }
}

impl ResourceFactory for MIDIResourceFactory {
    fn parse(&self, header: &ResourceHeader, raw: &[u8]) -> Result<Resource, String> {
        let smf = midly::Smf::parse(raw).map_err(|e| format!("Failed to parse MIDI: {}", e))?;

        // TODO: Implement proper MIDI parsing and event handling
        let clock_to_ms = |clocks_delta, tempo, signature| match smf.header.timing {
            midly::Timing::Metrical(tpqn) => {
                let tpqn = tpqn.as_int() as f32;
                let mpqn = tempo as f32 / 1_000_000.0; // Convert microseconds to seconds
                clocks_delta as f32 * (mpqn / tpqn * 1000.0) // Convert to milliseconds
            }
            midly::Timing::Timecode(fps, sub_frames) => {
                clocks_delta as f32 * (1000.0 / fps.as_f32() / sub_frames as f32)
            }
        };

        struct TrackEventWrapper<'a> {
            absolute_time: u32,
            delta: u32,
            kind: midly::TrackEventKind<'a>,
        }

        // Merge all track events into a single vector since we will not support multiple tracks
        let mut track_events = Vec::new();
        for track in smf.tracks.iter() {
            let mut absolute_time = 0;
            for event in track.iter() {
                absolute_time += event.delta.as_int();
                track_events.push(TrackEventWrapper {
                    absolute_time,
                    delta: event.delta.as_int(),
                    kind: event.kind.clone(),
                });
            }
        }
        // Sort events by absolute time
        track_events.sort_by_key(|e| e.absolute_time);
        // Recalculate deltas
        for i in 1..track_events.len() {
            track_events[i].delta = track_events[i].absolute_time - track_events[i - 1].absolute_time;
        }

        let mut events = Vec::new();
        let mut tempo: u32 = 0; // Default tempo in microseconds per quarter note
        let mut signature = (0, 0, 0, 0); // Default time signature (4/4)

        for event in track_events.iter() {
            if event.delta != 0 {
                // Convert delta time to milliseconds
                let ms = clock_to_ms(event.delta, tempo, signature);
                events.push(MIDIEvent::Idle { ms });
            }
            match event.kind {
                midly::TrackEventKind::Meta(midly::MetaMessage::Tempo(t)) => {
                    tempo = t.as_int();
                }
                midly::TrackEventKind::Meta(midly::MetaMessage::TimeSignature(
                    num,
                    denom,
                    clocks_per_tick,
                    notes_per_quarter,
                )) => {
                    signature = (num, denom, clocks_per_tick, notes_per_quarter);
                }
                midly::TrackEventKind::Midi { channel, message } => match message {
                    MidiMessage::NoteOff { key, vel } => {
                        events.push(MIDIEvent::NoteOff {
                            channel: channel.as_int(),
                            note: key.as_int(),
                        });
                    }
                    MidiMessage::NoteOn { key, vel } if vel.as_int() == 0 => {
                        // Note On with velocity 0 is equivalent to Note Off
                        events.push(MIDIEvent::NoteOff {
                            channel: channel.as_int(),
                            note: key.as_int(),
                        });
                    }
                    MidiMessage::NoteOn { key, vel } => {
                        events.push(MIDIEvent::NoteOn {
                            channel: channel.as_int(),
                            note: key.as_int(),
                            velocity: vel.as_int(),
                        });
                    }
                    _ => {
                        println!("Unhandled MIDI message: {:?}", message);
                    }
                },
                _ => {
                    println!("Unhandled MIDI event: {:?}", event.kind);
                }
            }
        }

        Ok(Resource::new(MIDIResource { events }))
    }

    fn finalize(&self, header: &ResourceHeader, resource: &Resource) -> Result<(), String> {
        Ok(())
    }
}

fn main() {
    // Initialize logging
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    const SAMPLE_RATE: usize = 44_100;

    let resource_manager = Arc::new(ResourceManager::new(ResourceManagerConfig {
        backend: Box::new(YARCResourceManagerIO::new("demo_sound.yarc".to_string())),
    }));
    resource_manager.register_factory(
        ResourceType::AudioWAV,
        Arc::new(WAVResourceFactory::new(SAMPLE_RATE)),
    );
    resource_manager.register_factory(
        ResourceType::AudioOGG,
        Arc::new(OGGResourceFactory::new(SAMPLE_RATE)),
    );
    resource_manager.register_factory(
        ResourceType::AudioFLAC,
        Arc::new(FLACResourceFactory::new(SAMPLE_RATE)),
    );
    resource_manager.register_factory(
        ResourceType::AudioMIDI,
        Arc::new(MIDIResourceFactory::new()),
    );

    resource_manager.poll_io().unwrap();

    let (mut controller, bus) =
        MidiPlayer::<24>::new(resource_manager.get_resource("beethoven").unwrap());

    let sink = InterleavedSink::new(bus, SAMPLE_RATE);

    let thread_manager_config = ThreadManagerConfig::default();
    let _ = scoped(thread_manager_config, |manager| {
        let config = PlayerConfig {
            thread_manager: &manager,
            backend_config: PlayerBackendConfig {},
            profiler: Some(profile_player),
            sample_rate: SAMPLE_RATE,
        };

        let player = Player::new(config, sink).unwrap();

        manager
            .spawn(
                "controller".to_string(),
                ThreadPriority::Normal,
                move || controller.play(&player),
            )
            .unwrap();

        // Player will be dropped here when the thread is finished.
        // Threads will be automatically joined when they go out of scope.
    });

    resource_manager.finalize_all(ResourceType::AudioWAV);

    info!("Yage2 Engine finished");
}
