mod chain;

use crate::chain::{Pass, PassEvents};
use common::assets::YARCReader;
use common::logging::CommonLogger;
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::world::World;
use glam::*;
use log::info;
use yage2_core::assets::factory::FactoryBinding;
use yage2_core::assets::hub::AssetHub;
use yage2_core::assets::AssetType;
use yage2_core::ecs::{run_loop, MainLoopProfileFrame, StopEventLoop};
use yage2_graphics::construct_chain;
use yage2_graphics::input::{InputEvent, KeyCode};
use yage2_graphics::passes::chain::ChainCons;
use yage2_graphics::passes::chain::ChainNil;
use yage2_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use yage2_graphics::passes::pipeline::RenderPipeline;
use yage2_graphics::renderable::{Position, RenderableMesh};
use yage2_graphics::renderer::{Renderer, RendererBackendConfig, RendererProfileFrame};
use yage2_graphics::view::{PlatformSpecificViewConfig, ViewConfig};

const REFRESH_RATE: f32 = 144.0;

#[derive(Component)]
struct GameController {
    pass_target_id: RenderPassTargetId,
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_asset_hub(world: &mut World) -> (FactoryBinding, FactoryBinding) {
        let reader = YARCReader::new("demo_graphics.yarc".to_string());
        let mut manager = AssetHub::new(reader).unwrap();

        // Unlike other factories, shader and texture assets are
        // managed directly by the renderer, instead of processing assets
        // in the main loop (via ECS).
        let shader_binding = manager.create_factory_biding(AssetType::ShaderSPIRV);
        let texture_binding = manager.create_factory_biding(AssetType::ImagePNG);

        manager.attach_to_ecs(world);

        (shader_binding, texture_binding)
    }

    pub fn setup_graphics(
        world: &mut World,
        shader_binding: FactoryBinding,
        texture_binding: FactoryBinding,
    ) -> RenderPassTargetId {
        let view_config = ViewConfig {
            platform_specific: PlatformSpecificViewConfig {},
            title: "Hello world".to_string(),
            width: 800,
            height: 600,
        };

        let pass = Pass::new();
        let pass_target_id = pass.get_id();

        let backend_config = RendererBackendConfig {
            fps: REFRESH_RATE as usize,
            pipeline: RenderPipeline::new(construct_chain!(pass)),
            shader_factory_binding: Some(shader_binding),
            texture_factory_binding: Some(texture_binding),
            vsync: true,
        };

        let renderer = Renderer::new(view_config, backend_config, true).unwrap();
        renderer.attach_to_ecs(world);

        pass_target_id
    }

    pub fn setup(world: &mut World) {
        let (shader_binding, texture_binding) = Self::setup_asset_hub(world);
        let id = Self::setup_graphics(world, shader_binding, texture_binding);
        GameController { pass_target_id: id }.attach_to_ecs(world);
    }
}

fn main_loop_profile_handler(r: Receiver<MainLoopProfileFrame>) {
    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tick_tps.average(),
        r.event.average_load * 100.0
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

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new();
    GameController::setup(&mut world);

    let quad = world.spawn();
    world.insert(
        quad,
        Position {
            0: Vec3::new(0.0, 0.0, 0.0),
        },
    );
    world.insert(quad, RenderableMesh { mesh_id: 0 });

    world.add_handler(main_loop_profile_handler);
    world.add_handler(renderer_profile_handler);
    world.add_handler(input_events_handler);

    world.add_handler(
        |ie: Receiver<InputEvent>,
         mut f: Fetcher<&mut Position>,
         gc: Single<&mut GameController>,
         mut s: Sender<RenderPassEvent<PassEvents>>| {
            for pos in f.iter_mut() {
                match ie.event {
                    InputEvent::MouseMove { x, y } => {
                        pos.0.x = x / 400.0 - 0.5; // Adjusting for screen size
                        pos.0.y = -y / 300.0 + 0.5; // Adjusting for screen size
                    }

                    InputEvent::KeyRelease(KeyCode::Space) => {
                        info!("Space key pressed, changing color");
                        let new_color = Vec3::new(
                            rand::random::<f32>(),
                            rand::random::<f32>(),
                            rand::random::<f32>(),
                        );
                        s.send(RenderPassEvent::new(
                            gc.pass_target_id,
                            PassEvents::ChangeColor(new_color),
                        ));
                    }
                    _ => {}
                }
            }
        },
    );

    run_loop(&mut world, REFRESH_RATE, true);
}
