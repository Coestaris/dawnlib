use common::logging::CommonLogger;
use common::resources::YARCResourceManagerIO;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::world::World;
use glam::*;
use log::info;
use std::sync::Arc;
use yage2_audio::backend::PlayerBackendConfig;
use yage2_audio::entities::bus::Bus;
use yage2_audio::entities::effects::soft_clip::SoftClipEffect;
use yage2_audio::entities::events::{AudioEvent, AudioEventTargetId, AudioEventType};
use yage2_audio::entities::sinks::InterleavedSink;
use yage2_audio::entities::sources::actor::{
    ActorID, ActorsSource, ActorsSourceEvent, DistanceGainFunction, DistanceLPFFunction,
};
use yage2_audio::player::{Player, PlayerProfileFrame};
use yage2_audio::resources::{
    FLACResourceFactory, MIDIResourceFactory, OGGResourceFactory, WAVResourceFactory,
};
use yage2_core::ecs::{run_loop, MainLoopProfileFrame, StopEventLoop, Tick};
use yage2_core::resources::{Resource, ResourceManager, ResourceManagerConfig, ResourceType};

#[cfg(target_os = "linux")]
// Alsa backend works A LOT better with 44,100 Hz sample rate
const SAMPLE_RATE: usize = 44100;
#[cfg(not(target_os = "linux"))]
const SAMPLE_RATE: usize = 48000;

const REFRESH_RATE: f32 = 60.0; // Main loop refresh rate in ticks per second

#[derive(Component)]
struct GameController {
    actors_source_target: AudioEventTargetId,
    resource_manager: ResourceManager,
    counter: usize,
}

impl Drop for GameController {
    fn drop(&mut self) {
        info!("GameController dropped");
        self.resource_manager.finalize_all(ResourceType::AudioWAV);
    }
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_resource_manager() -> ResourceManager {
        // Setup resource manager
        let resource_manager = ResourceManager::new(ResourceManagerConfig {
            backend: Box::new(YARCResourceManagerIO::new("demo_audio.yarc".to_string())),
        });
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
        resource_manager
    }

    fn setup_audio_pipeline(world: &mut World) -> AudioEventTargetId {
        // Setup Audio Pipeline
        let actors_source = ActorsSource::new(
            DistanceGainFunction::Constant(1.0),
            DistanceLPFFunction::Constant(0.0),
        );
        let actors_source_target = actors_source.get_id();
        let clipper = SoftClipEffect::new(0.5, 0.1);
        let bus = Bus::new(clipper, actors_source, None, None);

        // Start Audio Player (output)
        let sink = InterleavedSink::new(bus, SAMPLE_RATE);
        let player = Player::new(SAMPLE_RATE, PlayerBackendConfig {}, sink, true).unwrap();

        // Allow player to receive events from the ECS
        // (moved to the ECS storage)
        player.attach_to_ecs(world);

        actors_source_target
    }

    pub fn setup(world: &mut World) {
        // Move controller to the ECS
        GameController {
            actors_source_target: Self::setup_audio_pipeline(world),
            resource_manager: Self::setup_resource_manager(),
            counter: 0,
        }
        .attach_to_ecs(world);
    }

    pub fn add_audio_actor(&self, pos: Vec3, gain: f32, clip: Resource) -> (AudioEvent, ActorID) {
        let actor_id = ActorID::new();
        (
            AudioEvent::new(
                self.actors_source_target,
                AudioEventType::Actors(ActorsSourceEvent::AddActor {
                    id: Some(actor_id.clone()),
                    pos,
                    gain,
                    clip, // This should be set to a valid audio clip
                }),
            ),
            actor_id,
        )
    }
}

fn timeout_handler(t: Receiver<Tick>, mut stopper: Sender<StopEventLoop>) {
    if t.event.time > 10.0 {
        // Stop the event loop after 10 seconds
        stopper.send(StopEventLoop);
    }
}

fn main_loop_profile_handler(r: Receiver<MainLoopProfileFrame>) {
    let allowed_time = 1000.0 / REFRESH_RATE;

    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tick_tps.average(),
        r.event.tick_time.average() / allowed_time * 1000.0
    );
}

fn player_profile_handler(r: Receiver<PlayerProfileFrame>) {
    let frame = r.event;

    // Number of samples that actually processed by one render call
    // (assuming that no underruns happens).
    let av_actual_samples = frame.sample_rate as f32 / frame.render_tps.average();
    // Calculate the allowed time for one render call
    let allowed_time = av_actual_samples / frame.sample_rate as f32 * 1000.0;

    // When no events are processed, we cannot calculate the load
    // (since the thread is not running).
    // Assume that the events thread has the same maximum allowed time
    // as the renderer thread.
    let events_load_precent = if frame.events_tps.average() == 0.0 {
        0.0
    } else {
        frame.events.average() / allowed_time * 100.0
    };

    info!(
        "T: {:.0}. Render: {:.1}ms ({:.1}%). Ev {:.1}ms ({:.1}%) ({:.0})",
        frame.render_tps.average(),
        frame.render.average(),
        frame.render.average() / allowed_time * 100.0,
        frame.events.average(),
        events_load_precent,
        frame.events_tps.average(),
    );
}

fn player_handler(
    _: Receiver<Tick>,
    mut gc: Single<&mut GameController>,
    mut sender: Sender<AudioEvent>,
) {
    if gc.counter == 0 {
        let clip = gc.resource_manager.get_resource("loop").unwrap();
        sender.send(gc.add_audio_actor(Vec3::ZERO, 0.7, clip).0);
        gc.counter += 1;
    }
}

fn main() {
    // Initialize logging
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();
    GameController::setup(&mut world);

    world.add_handler(timeout_handler);
    world.add_handler(player_handler);
    world.add_handler(main_loop_profile_handler);
    world.add_handler(player_profile_handler);

    run_loop(&mut world, REFRESH_RATE, true);
}
