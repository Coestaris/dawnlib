use common::logging::CommonLogger;
use common::resources::YARCResourceManagerIO;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::world::World;
use log::info;
use yage2_core::ecs::{run_loop, MainLoopProfileFrame, StopEventLoop, Tick};
use yage2_core::resources::{ResourceManager, ResourceManagerConfig, ResourceType};
use yage2_graphics::input::{InputEvent, KeyCode};
use yage2_graphics::renderer::{Renderer, RendererBackendConfig, RendererProfileFrame};
use yage2_graphics::view::{PlatformSpecificViewConfig, ViewConfig};

const REFRESH_RATE: f32 = 60.0;

#[derive(Component)]
struct GameController {
    resource_manager: ResourceManager,
}

impl Drop for GameController {
    fn drop(&mut self) {
        info!("GameController dropped");
        self.resource_manager
            .finalize_all(ResourceType::ShaderSPIRV);
    }
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_graphics(world: &mut World) {
        let view_config = ViewConfig {
            platform_specific: PlatformSpecificViewConfig {},
            title: "Hello world".to_string(),
            width: 800,
            height: 600,
        };
        let backend_config = RendererBackendConfig {};
        let renderer = Renderer::new(view_config, backend_config, true).unwrap();
        renderer.attach_to_ecs(world);
    }

    pub fn setup_resource_manager() -> ResourceManager {
        // Setup resource manager
        let resource_manager = ResourceManager::new(ResourceManagerConfig {
            backend: Box::new(YARCResourceManagerIO::new("demo_graphics.yarc".to_string())),
        });
        resource_manager.poll_io().unwrap();
        resource_manager
    }

    pub fn setup(world: &mut World) {
        Self::setup_graphics(world);
        GameController {
            resource_manager: Self::setup_resource_manager(),
        }
        .attach_to_ecs(world);
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

fn renderer_profile_handler(r: Receiver<RendererProfileFrame>) {
    info!(
        "Renderer: {:.1} FPS. {:.1}/{:.1}",
        r.event.fps.average(),
        r.event.backend_tick.average(),
        r.event.view_tick.average()
    );
}

fn input_events_handler(r: Receiver<InputEvent>, mut s: Sender<StopEventLoop>) {
    // info!("Input event: {:?}", r.event);
    if let InputEvent::KeyRelease(KeyCode::Escape) = r.event {
        info!("Escape key pressed, stopping the event loop");
        s.send(StopEventLoop);
    }
}

fn timeout_handler(t: Receiver<Tick>, mut stopper: Sender<StopEventLoop>) {
    if t.event.time > 10.0 {
        // Stop the event loop after 10 seconds
        stopper.send(StopEventLoop);
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();
    GameController::setup(&mut world);

    world.add_handler(main_loop_profile_handler);
    world.add_handler(renderer_profile_handler);
    world.add_handler(input_events_handler);
    world.add_handler(timeout_handler);

    run_loop(&mut world, REFRESH_RATE, true);
}
