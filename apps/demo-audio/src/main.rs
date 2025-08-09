use common::assets::YARCReader;
use common::logging::CommonLogger;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::Single;
use evenio::world::World;
use glam::*;
use log::info;
use yage2_audio::assets::{MIDIAssetFactory, WAVAssetFactory};
use yage2_audio::backend::PlayerBackendConfig;
use yage2_audio::entities::bus::Bus;
use yage2_audio::entities::effects::soft_clip::SoftClipEffect;
use yage2_audio::entities::events::{AudioEvent, AudioEventTargetId, AudioEventType};
use yage2_audio::entities::sinks::InterleavedSink;
use yage2_audio::entities::sources::actor::{
    ActorID, ActorsSource, ActorsSourceEvent, DistanceGainFunction, DistanceLPFFunction,
};
use yage2_audio::player::{Player, PlayerProfileFrame};
use yage2_core::assets::hub::{AssetHub, AssetHubEvent};
use yage2_core::assets::{Asset, AssetID, AssetType};
use yage2_core::ecs::{run_loop, MainLoopProfileFrame, StopEventLoop, Tick};

#[cfg(target_os = "linux")]
// Alsa backend works A LOT better with 44,100 Hz sample rate
const SAMPLE_RATE: usize = 44100;
#[cfg(not(target_os = "linux"))]
const SAMPLE_RATE: usize = 48000;

const REFRESH_RATE: f32 = 60.0; // Main loop refresh rate in ticks per second

#[derive(Component)]
struct GameController {
    actors_source_target: AudioEventTargetId,
    counter: usize,
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_asset_hub(world: &mut World) {
        // Setup asset hub
        let reader = YARCReader::new("demo_audio.yarc".to_string());
        let mut hub = AssetHub::new(reader).unwrap();

        let mut wav_factory = WAVAssetFactory::new(SAMPLE_RATE);
        wav_factory.bind(hub.create_factory_biding(AssetType::AudioWAV));
        wav_factory.attach_to_ecs(world);

        let mut midi_factory = MIDIAssetFactory::new();
        midi_factory.bind(hub.create_factory_biding(AssetType::AudioMIDI));
        midi_factory.attach_to_ecs(world);

        hub.query_load_all().unwrap();
        hub.attach_to_ecs(world);
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

        // Allow player to receive events from the ECS (moved to the ECS storage)
        player.attach_to_ecs(world);

        actors_source_target
    }

    pub fn setup(world: &mut World) {
        Self::setup_asset_hub(world);

        // Move controller to the ECS
        GameController {
            actors_source_target: Self::setup_audio_pipeline(world),
            counter: 0,
        }
        .attach_to_ecs(world);
    }

    pub fn add_audio_actor(&self, pos: Vec3, gain: f32, clip: Asset) -> (AudioEvent, ActorID) {
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
    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tick_tps.average(),
        r.event.average_load * 100.0
    );
}

fn player_profile_handler(r: Receiver<PlayerProfileFrame>) {
    let frame = r.event;
    info!(
        "T: {:.0}. Render: {:.1}ms ({:.1}%). Ev {:.1}ms ({:.1}%) ({:.0})",
        frame.render_tps.average(),
        frame.render.average(),
        frame.average_renderer_load * 100.0,
        frame.events.average(),
        frame.average_events_load * 100.0,
        frame.events_tps.average(),
    );
}

fn player_handler(
    e: Receiver<AssetHubEvent>,
    mut gc: Single<&mut GameController>,
    mut rm: Single<&mut AssetHub>,
    mut sender: Sender<AudioEvent>,
) {
    if matches!(e.event, AssetHubEvent::AllAssetsLoaded) {
        let clip = rm.get(AssetID::new("loop".to_string())).unwrap();
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
