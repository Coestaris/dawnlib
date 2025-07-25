mod notes;

use crate::notes::{Note, NoteName};
use common::logging::CommonLogger;
use common::profilers::profile_threads;
use common::resources::YARCResourceManagerIO;
use log::info;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use yage2_core::resources::{ResourceManager, ResourceManagerConfig, ResourceType};
use yage2_core::threads::{scoped, ThreadManagerConfig, ThreadPriority};
use yage2_sound::backend::PlayerBackendConfig;
use yage2_sound::entities::bus::Bus;
use yage2_sound::entities::effects::bypass::BypassEffect;
use yage2_sound::entities::events::{Event, EventBox, EventTargetId};
use yage2_sound::entities::sinks::InterleavedSink;
use yage2_sound::entities::sources::multiplexer::{Multiplexer3Source, MultiplexerSource};
use yage2_sound::entities::sources::waveform::{WaveformSource, WaveformSourceEvent, WaveformType};
use yage2_sound::player::{Player, PlayerConfig};
use yage2_sound::resources::{FLACResourceFactory, OGGResourceFactory, WAVResourceFactory};

fn profile_audio(frame: &yage2_sound::player::ProfileFrame) {
    // Calculate the time in milliseconds, the renderer thread
    // is maximally allowed to take to fill the device buffer.
    let allowed_time = (1000.0 / frame.sample_rate as f32) * frame.block_size as f32;

    // When no events are processed, we cannot calculate the load
    // (since the thread is not running).
    // Assume that events thread has the same maximum allowed time
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

struct MidiPlayer<const VOICES_COUNT: usize> {
    ids: [EventTargetId; VOICES_COUNT],
}

impl<const VOICES_COUNT: usize> MidiPlayer<VOICES_COUNT> {
    fn new<'a>() -> (
        MidiPlayer<VOICES_COUNT>,
        Bus<'a, BypassEffect, MultiplexerSource<'a, WaveformSource, VOICES_COUNT>>,
    ) {
        fn leak<T>(value: T) -> &'static T {
            Box::leak(Box::new(value))
        }
        let sources: [&WaveformSource; VOICES_COUNT] =
            std::array::from_fn(|_| leak(WaveformSource::new(None, None, None)));
        let ids: [EventTargetId; VOICES_COUNT] = sources
            .iter()
            .map(|source| source.get_id())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let gain: f32 = 3.0 / VOICES_COUNT as f32;
        let mixes: [f32; VOICES_COUNT] = [gain; VOICES_COUNT];
        let multiplexer = leak(MultiplexerSource::new(sources, mixes));
        let effect = leak(BypassEffect::new());

        (MidiPlayer { ids }, Bus::new(effect, multiplexer))
    }

    fn set_freq(&mut self, player: &Player, id: usize, name: NoteName, octave: u8) {
        if id >= VOICES_COUNT {
            panic!("Invalid voice ID: {}", id);
        }
        let frequency = Note::new(name, octave).frequency();
        let event = WaveformSourceEvent::SetFrequency(frequency);
        let event = EventBox::new(self.ids[id], Event::Waveform(event));
        player.push_event(&event);

        let event = WaveformSourceEvent::SetWaveformType(WaveformType::Sine);
        let event = EventBox::new(self.ids[id], Event::Waveform(event));
        player.push_event(&event);
    }

    fn play(&mut self, player: &Player) {
        self.set_freq(player, 0, NoteName::C, 4);
        self.set_freq(player, 1, NoteName::E, 4);
        self.set_freq(player, 2, NoteName::G, 4);

        sleep(Duration::from_secs(2));

        self.set_freq(player, 0, NoteName::C, 4);
        self.set_freq(player, 1, NoteName::B, 4);
        self.set_freq(player, 2, NoteName::A, 4);

        sleep(Duration::from_secs(2));

        self.set_freq(player, 0, NoteName::C, 4);
        self.set_freq(player, 1, NoteName::E, 4);
        self.set_freq(player, 2, NoteName::G, 4);

        sleep(Duration::from_secs(2));
    }
}

fn main() {
    // Initialize logging
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

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

    resource_manager.poll_io().unwrap();

    let (mut controller, bus) = MidiPlayer::<8>::new();
    let sink = InterleavedSink::new(bus, SAMPLE_RATE);

    let thread_manager_config = ThreadManagerConfig::default();
    let _ = scoped(thread_manager_config, |manager| {
        let config = PlayerConfig {
            thread_manager: &manager,
            backend_config: PlayerBackendConfig {},
            profiler_handler: Some(profile_audio),
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
