use common::logging::CommonLogger;
use common::resources::YARCResourceManagerIO;
use log::info;
use std::sync::Arc;
use evenio::world::World;
use yage2_audio::backend::PlayerBackendConfig;
use yage2_audio::entities::bus::Bus;
use yage2_audio::entities::effects::bypass::BypassEffect;
use yage2_audio::entities::effects::soft_clip::SoftClipEffect;
use yage2_audio::entities::sinks::InterleavedSink;
use yage2_audio::entities::sources::actor::ActorsSource;
use yage2_audio::player::{Player, PlayerConfig, ProfileFrame};
use yage2_audio::resources::{
    FLACResourceFactory, MIDIResourceFactory, OGGResourceFactory, WAVResourceFactory,
};
use yage2_core::resources::{ResourceManager, ResourceManagerConfig, ResourceType};

#[cfg(target_os = "linux")]
// Alsa backend works A LOT better with 44,100 Hz sample rate
const SAMPLE_RATE: usize = 44100;
#[cfg(not(target_os = "linux"))]
const SAMPLE_RATE: usize = 48000;

fn profile_player(frame: &ProfileFrame) {
    // Number of samples that actually processed by one render call
    // (assuming that no underruns happens).
    let av_actual_samples = frame.sample_rate as f32 / frame.render_tps_av as f32;
    // Calculate the allowed time for one render call
    let allowed_time = av_actual_samples / frame.sample_rate as f32 * 1000.0;

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

fn main() {
    // Initialize logging
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    // Setup resource manager
    let resource_manager = Arc::new(ResourceManager::new(ResourceManagerConfig {
        backend: Box::new(YARCResourceManagerIO::new("demo_audio.yarc".to_string())),
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

    let mut world = World::new();

    // Setup Audio Pipeline
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    let actors_source = leak(ActorsSource::new());
    let clipper = leak(BypassEffect::new());
    let bus = Bus::new(clipper, actors_source, None, None);

    // Start Audio Player (output)
    let sink = InterleavedSink::new(bus, SAMPLE_RATE);
    let config = PlayerConfig {
        backend_config: PlayerBackendConfig {},
        profiler: Some(profile_player),
        sample_rate: SAMPLE_RATE,
    };
    let player = Player::new(config, sink).unwrap();
    // Allow player to receive events from the ECS
    player.attach_to_ecs(&mut world);

    let entity = world.spawn();
}
