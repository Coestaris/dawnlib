use crate::sample::Sample;
use crate::{ChannelsCount, SampleRate, SamplesCount, CHANNELS_COUNT};

// Converts interleaved raw audio samples to planar f32 samples.
fn to_planar_f32<F>(
    interleaved_raw: Vec<F>,
    raw_channels: ChannelsCount,
) -> [Vec<f32>; CHANNELS_COUNT]
where
    F: Sample + Copy,
{
    let mut output = [
        Vec::with_capacity(interleaved_raw.len() / raw_channels),
        Vec::with_capacity(interleaved_raw.len() / raw_channels),
    ];

    let mut i = 0;
    while i < interleaved_raw.len() {
        // Our interleaved sample has a fixed number of channels that
        // can be less or greater than the input channels.
        // In case of fewer channels, we will just copy last sample to the rest of the channels.
        // In case of more channels, we will just ignore the extra channels.
        for j in 0..CHANNELS_COUNT {
            let val = if j < raw_channels {
                F::to_f32(interleaved_raw[i + j])
            } else {
                // If we have fewer channels, copy the last sample to the rest of the channels
                F::to_f32(interleaved_raw[i + raw_channels - 1])
            };
            output[j].push(val);
        }

        // Stride the input by the number of channels
        i += raw_channels;
    }

    output
}

pub struct AudioAsset {
    pub sample_rate: SampleRate,
    pub len: SamplesCount,
    pub channels: ChannelsCount,
    pub data: [Vec<f32>; CHANNELS_COUNT],
}

#[cfg(feature = "wav-assets")]
pub(crate) mod wav {
    use crate::assets::{to_planar_f32, AudioAsset};
    use crate::{ChannelsCount, SampleRate, CHANNELS_COUNT};
    use evenio::component::Component;
    use evenio::event::Receiver;
    use evenio::fetch::Single;
    use evenio::prelude::World;
    use log::error;
    use std::io::Cursor;
    use yage2_core::assets::factory::{BasicFactory, FactoryBinding};
    use yage2_core::assets::reader::AssetHeader;
    use yage2_core::assets::AssetType;
    use yage2_core::ecs::Tick;

    #[derive(Component)]
    pub struct WAVAssetFactory {
        sample_rate: SampleRate,
        basic_factory: BasicFactory<AudioAsset>,
    }

    impl WAVAssetFactory {
        pub fn new(sample_rate: SampleRate) -> Self {
            WAVAssetFactory {
                sample_rate,
                basic_factory: BasicFactory::new(),
            }
        }

        pub fn bind(&mut self, binding: FactoryBinding) {
            assert_eq!(binding.asset_type(), AssetType::AudioWAV);
            self.basic_factory.bind(binding);
        }

        pub fn process_events(&mut self) {
            self.basic_factory
                .process_events(|header, raw| parse(self.sample_rate, &header, &raw), |_| {});
        }

        pub fn attach_to_ecs(self, world: &mut World) {
            // This factory can be attached to the ECS as a component
            // to allow processing events in the game loop.
            let entity = world.spawn();
            world.insert(entity, self);

            fn tick_handler(_: Receiver<Tick>, mut factory: Single<&mut WAVAssetFactory>) {
                factory.process_events();
            }

            world.add_handler(tick_handler);
        }
    }

    fn parse(sample_rate: SampleRate, header: &AssetHeader, raw: &[u8]) -> Option<AudioAsset> {
        let mut buf_reader = Cursor::new(raw);
        match hound::WavReader::new(&mut buf_reader) {
            Ok(mut reader) => {
                let spec = reader.spec();
                if spec.sample_rate as SampleRate != sample_rate {
                    error!(
                            "WAV {} sample rate mismatch: expected {}, got {}. Resampling is currently not supported.",
                            header.name, sample_rate, spec.sample_rate
                        );
                    return None;
                }

                let data = match (spec.sample_format, spec.bits_per_sample) {
                    (hound::SampleFormat::Float, 32) => {
                        let samples: Vec<f32> =
                            reader.samples::<f32>().map(|s| s.unwrap()).collect();
                        to_planar_f32(samples, spec.channels as ChannelsCount)
                    }
                    (hound::SampleFormat::Int, 16) => {
                        let samples: Vec<i16> =
                            reader.samples::<i16>().map(|s| s.unwrap()).collect();
                        to_planar_f32(samples, spec.channels as ChannelsCount)
                    }
                    (hound::SampleFormat::Int, 24) => {
                        let samples: Vec<i24::i24> = reader
                            .samples::<i32>()
                            .map(|s| i24::i24::try_from_i32(s.unwrap()).unwrap())
                            .collect();
                        to_planar_f32(samples, spec.channels as ChannelsCount)
                    }
                    (hound::SampleFormat::Int, 32) => {
                        let samples: Vec<i32> =
                            reader.samples::<i32>().map(|s| s.unwrap()).collect();
                        to_planar_f32(samples, spec.channels as ChannelsCount)
                    }
                    _ => {
                        error!(
                            "Unsupported WAV {} format: {:?} with {} bits per sample",
                            header.name, spec.sample_format, spec.bits_per_sample
                        );
                        return None;
                    }
                };

                Some(AudioAsset {
                    sample_rate,
                    len: data[0].len(),
                    channels: CHANNELS_COUNT,
                    data,
                })
            }

            Err(e) => {
                error!("Failed to read WAV {}: {}", header.name, e);
                None
            }
        }
    }
}

#[cfg(feature = "flac-assets")]
pub(crate) mod flac {}

#[cfg(feature = "ogg-assets")]
pub(crate) mod ogg {}

pub(crate) mod midi {
    use evenio::component::Component;
    use evenio::event::Receiver;
    use evenio::fetch::Single;
    use evenio::world::World;
    use midly::MidiMessage;
    use yage2_core::assets::factory::{BasicFactory, FactoryBinding};
    use yage2_core::assets::reader::AssetHeader;
    use yage2_core::assets::AssetType;
    use yage2_core::ecs::Tick;

    pub enum MIDIEvent {
        NoteOn { channel: u8, note: u8, velocity: u8 },
        NoteOff { channel: u8, note: u8 },
        Idle { ms: f32 },
    }

    pub struct MIDIAsset {
        pub events: Vec<MIDIEvent>,
    }

    impl MIDIAsset {
        pub fn new(events: Vec<MIDIEvent>) -> Self {
            MIDIAsset { events }
        }
    }

    #[derive(Component)]
    pub struct MIDIAssetFactory {
        basic_factory: BasicFactory<MIDIAsset>,
    }

    impl MIDIAssetFactory {
        pub fn new() -> Self {
            MIDIAssetFactory {
                basic_factory: BasicFactory::new(),
            }
        }

        pub fn bind(&mut self, binding: FactoryBinding) {
            assert_eq!(binding.asset_type(), AssetType::AudioMIDI);
            self.basic_factory.bind(binding);
        }

        pub fn process_events(&mut self) {
            self.basic_factory
                .process_events(|header, raw| parse(&header, &raw), |_| {});
        }

        pub fn attach_to_ecs(self, world: &mut World) {
            // This factory can be attached to the ECS as a component
            // to allow processing events in the game loop.
            let entity = world.spawn();
            world.insert(entity, self);

            fn tick_handler(_: Receiver<Tick>, mut factory: Single<&mut MIDIAssetFactory>) {
                factory.process_events();
            }

            world.add_handler(tick_handler);
        }
    }

    fn parse(_: &AssetHeader, raw: &[u8]) -> Option<MIDIAsset> {
        let smf = midly::Smf::parse(raw)
            .map_err(|e| format!("Failed to parse MIDI: {}", e))
            .ok()?;

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
            track_events[i].delta =
                track_events[i].absolute_time - track_events[i - 1].absolute_time;
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
                        // println!("Unhandled MIDI message: {:?}", message);
                    }
                },
                _ => {
                    // println!("Unhandled MIDI event: {:?}", event.kind);
                }
            }
        }

        Some(MIDIAsset { events })
    }
}

pub use {
    midi::{MIDIAsset, MIDIAssetFactory, MIDIEvent},
    wav::WAVAssetFactory,
};
