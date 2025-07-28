use crate::objects::{EventListener, Point, SimpleObject};
use common::logging::CommonLogger;
use common::profilers::profile_threads;
use common::resources::YARCResourceManagerIO;
use log::info;
use std::sync::Arc;
use yage2_app::view::{PlatformSpecificViewConfig, ViewConfig};
use yage2_app::{create_object, GraphicsConfig};
use yage2_core::resources::{ResourceManager, ResourceManagerConfig};
use yage2_core::threads::{ThreadManager, ThreadManagerConfig};

mod objects;

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    // let resource_manager = Arc::new(ResourceManager::new(ResourceManagerConfig {
    //     backend: Box::new(YARCResourceManagerIO::new("demo_app.yarc".to_string())),
    // }));
    // resource_manager.poll_io().unwrap();
    //
    // let thread_manager: Arc<ThreadManager> = Arc::new(ThreadManager::new(ThreadManagerConfig {
    //     profile_handle: Some(profile_threads),
    // }));
    //
    // let application_config = ApplicationConfig {
    //     thread_manager: thread_manager.clone(),
    //     startup_objects: vec![
    //         create_object!(EventListener),
    //         create_object!(SimpleObject::new(Point::new(10.0, 200.0))),
    //         create_object!(SimpleObject::new(Point::new(100.0, 200.0))),
    //         create_object!(SimpleObject::new(Point::new(200.0, 200.0))),
    //     ],
    //     resource_manager: resource_manager.clone(),
    //     graphics_config: GraphicsConfig {},
    //     view_config: ViewConfig {
    //         platform_specific: PlatformSpecificViewConfig {},
    //         title: "Yage2 Engine".to_string(),
    //         width: 1280,
    //         height: 720,
    //     },
    // };
    // let app = Application::new(application_config).expect("Failed to create application");
    // app.run().expect("Failed to run application");
    // thread_manager.join_all();
}
