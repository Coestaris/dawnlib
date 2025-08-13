mod chain;

use crate::chain::{AABBPass, CustomPassEvent, GeometryPass};
use common::logging::{format_system_time, CommonLogger};
use evenio::component::Component;
use evenio::event::{Receiver, Sender};
use evenio::fetch::{Fetcher, Single};
use evenio::world::World;
use glam::*;
use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use yage2_core::assets::factory::FactoryBinding;
use yage2_core::assets::hub::AssetHub;
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::reader::AssetReader;
use yage2_core::assets::{AssetHeader, AssetID, AssetType};
use yage2_core::ecs::{run_loop, run_loop_with_monitoring, MainLoopMonitoring, StopEventLoop};
use yage2_graphics::construct_chain;
use yage2_graphics::input::{InputEvent, KeyCode};
use yage2_graphics::passes::chain::ChainCons;
use yage2_graphics::passes::chain::ChainNil;
use yage2_graphics::passes::events::{RenderPassEvent, RenderPassTargetId};
use yage2_graphics::passes::pipeline::RenderPipeline;
use yage2_graphics::renderable::{Position, RenderableMesh};
use yage2_graphics::renderer::{Renderer, RendererBackendConfig, RendererMonitoring};
use yage2_graphics::view::{PlatformSpecificViewConfig, ViewConfig};

// On my linux machine, the refresh rate is 60 Hz.
// I'll deal with it later
#[cfg(target_os = "linux")]
const REFRESH_RATE: f32 = 60.0;
#[cfg(not(target_os = "linux"))]
const REFRESH_RATE: f32 = 144.0;

#[derive(Component)]
struct GameController {
    geometry_pass_id: RenderPassTargetId,
    aabb_pass_id: RenderPassTargetId,
}

impl GameController {
    fn attach_to_ecs(self, world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, self);
    }

    pub fn setup_asset_hub(world: &mut World) -> (FactoryBinding, FactoryBinding) {
        struct Reader;
        impl AssetReader for Reader {
            fn read(&mut self) -> Result<HashMap<AssetID, (AssetHeader, AssetRaw)>, String> {
                let yarc = env!("YARC_FILE");
                info!("Reading assets from: {}", yarc);

                let (manifest, assets) = yage2_yarc::read(PathBuf::from(yarc)).unwrap();
                info!("> Version: {}", manifest.version.unwrap_or("unknown".to_string()));
                info!("> Author: {}", manifest.author.unwrap_or("unknown".to_string()));
                info!("> Description: {}", manifest.description.unwrap_or("No description".to_string()));
                info!("> License: {}", manifest.license.unwrap_or("No license".to_string()));
                info!("> Created: {}", format_system_time(manifest.created).unwrap());
                info!("> Tool: {} (version {})", manifest.tool, manifest.tool_version);
                info!("> Serialzer: {} (version {})", manifest.serializer, manifest.serializer_version);
                info!("> Assets: {}", assets.len());

                let mut result = HashMap::new();
                for (header, raw) in assets {
                    result.insert(header.id.clone(), (header, raw));
                }

                Ok(result)
            }
        }
        let mut hub = AssetHub::new(Reader).unwrap();

        // Unlike other factories, shader and texture assets are
        // managed directly by the renderer, instead of processing assets
        // in the main loop (via ECS).
        let shader_binding = hub.create_factory_biding(AssetType::Shader);
        let texture_binding = hub.create_factory_biding(AssetType::Texture);

        hub.attach_to_ecs(world);

        (shader_binding, texture_binding)
    }

    pub fn setup_graphics(
        world: &mut World,
        shader_binding: FactoryBinding,
        texture_binding: FactoryBinding,
    ) -> (RenderPassTargetId, RenderPassTargetId) {
        let view_config = ViewConfig {
            platform_specific: PlatformSpecificViewConfig {},
            title: "Hello world".to_string(),
            width: 800,
            height: 600,
        };

        let backend_config = RendererBackendConfig {
            fps: REFRESH_RATE as usize,
            shader_factory_binding: Some(shader_binding),
            texture_factory_binding: Some(texture_binding),
            vsync: true,
        };

        let geometry_pass_id = RenderPassTargetId::new();
        let aabb_pass_id = RenderPassTargetId::new();

        let renderer = Renderer::new_with_monitoring(view_config, backend_config, move || {
            let geometry_pass = GeometryPass::new(geometry_pass_id);
            let aabb_pass = AABBPass::new(aabb_pass_id);
            Ok(RenderPipeline::new(construct_chain!(
                geometry_pass,
                aabb_pass
            )))
        })
        .unwrap();
        renderer.attach_to_ecs(world);

        (geometry_pass_id, aabb_pass_id)
    }

    pub fn setup(world: &mut World) {
        let (shader_binding, texture_binding) = Self::setup_asset_hub(world);
        let (geometry_pass, aabb_pass) =
            Self::setup_graphics(world, shader_binding, texture_binding);
        GameController {
            geometry_pass_id: geometry_pass,
            aabb_pass_id: aabb_pass,
        }
        .attach_to_ecs(world);
    }
}

fn main_loop_profile_handler(r: Receiver<MainLoopMonitoring>) {
    info!(
        "Main loop: {:.1}tps ({:.1}%)",
        r.event.tps.average(),
        r.event.load.average() * 100.0
    );
}

fn renderer_profile_handler(r: Receiver<RendererMonitoring>) {
    info!(
        "Renderer: {:.1} FPS. {:.1}/{:.1}",
        r.event.fps.average(),
        r.event.render.average().as_millis(),
        r.event.view.average().as_millis(),
    );
}

fn input_events_handler(r: Receiver<InputEvent>, mut s: Sender<StopEventLoop>) {
    // info!("Input event: {:?}", r.event);
    if let InputEvent::KeyRelease(KeyCode::Escape) = r.event {
        info!("Escape key pressed, stopping the event loop");
        s.send(StopEventLoop);
    }
}

fn events_handler(
    ie: Receiver<InputEvent>,
    mut f: Fetcher<&mut Position>,
    gc: Single<&mut GameController>,
    mut s: Sender<RenderPassEvent<CustomPassEvent>>,
) {
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
                    gc.geometry_pass_id,
                    CustomPassEvent::ChangeColor(new_color),
                ));
            }
            _ => {}
        }
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
    world.add_handler(events_handler);

    run_loop_with_monitoring(&mut world, REFRESH_RATE);
}
