use crate::log_prelude;
use crate::object::ObjectPtr;
use crate::object_collection::ObjectsCollection;
use crate::threads::{
    logic, renderer, statistics, LogicProfiler, LogicThreadConfig, RendererProfiler,
    RendererThreadConfig, StatisticsThreadConfig,
};
use crate::view::ViewConfig;
use crate::vulkan::graphics::GraphicsConfig;
use log::info;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use yage2_core::profile::TickProfiler;
use yage2_core::resources::ResourceManager;
use yage2_core::sync::Rendezvous;
use yage2_core::threads::{ThreadManager, ThreadPriority};

pub struct ApplicationConfig {
    pub thread_manager: Arc<ThreadManager>,
    pub resource_manager: Arc<ResourceManager>,

    pub graphics_config: GraphicsConfig,
    pub view_config: ViewConfig,

    pub startup_objects: Vec<ObjectPtr>, // TODO: Implement 'world' object
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApplicationError {
    LogicThreadStartError,
    RendererThreadStartError,
    StatisticsThreadStartError,
}

pub struct Application {
    thread_manager: Arc<ThreadManager>,
    startup_objects: Vec<ObjectPtr>,

    resource_manager: Arc<ResourceManager>,
    view_config: ViewConfig,

    graphics_config: GraphicsConfig,

    stop_signal: Arc<AtomicBool>,
}

impl Application {
    pub fn new(config: ApplicationConfig) -> Result<Application, ApplicationError> {
        Ok(Application {
            thread_manager: config.thread_manager,
            startup_objects: config.startup_objects,
            resource_manager: config.resource_manager,
            view_config: config.view_config,
            graphics_config: config.graphics_config,
            stop_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn run(&self) -> Result<(), ApplicationError> {
        log_prelude();

        //
        // Threading model of the engine:
        //            Sync                                     Sync
        // Renderer:   ||  Drawing scene                        || Copy renderables ||
        // Logic:      ||  Process input | Update objects | etc ||
        // Statistics:          |        Gather statistics             |
        //
        // - Renderer thread is responsible for rendering the scene and
        //   copying renderable objects to the renderer.
        // - Logic thread is responsible for processing input events,
        //   updating app logic, audio processing, etc.
        // - Statistics thread is responsible for gathering statistics.
        //   It runs only in debug mode
        //
        // All threads synchronize with each other using rendezvous points
        // (before_frame and after_frame). The renderer thread waits for the logic thread
        // to finish processing input and updating objects before it starts drawing the scene.
        //
        // Create barriers for synchronization
        let before_frame = Arc::new(Rendezvous::new(2));
        let after_frame = Arc::new(Rendezvous::new(2));

        // Create a channel for receiving events from the window handler that is usually handled
        // by the OS or renderer thread and sending them to the logic thread.
        let (events_sender, events_receiver) = std::sync::mpsc::channel();

        //
        let renderer_objects = Arc::new(Mutex::new(Vec::new()));
        let logic_objects = Arc::new(Mutex::new(ObjectsCollection::new(
            self.startup_objects.clone(),
        )));

        // Create the renderer profiler used to gather statistics and debug information
        let renderer_profiler = Arc::new(RendererProfiler {
            fps: TickProfiler::new(0.3),
            renderables: TickProfiler::new(1.0),
            drawn_triangles: TickProfiler::new(1.0),
            ..Default::default()
        });
        let logic_profiler = Arc::new(LogicProfiler {
            ups: TickProfiler::new(0.3),
            eps: Arc::new(TickProfiler::new(1.0)),
            ..Default::default()
        });

        // Starting the logic thread
        let logic_config = LogicThreadConfig {
            before_frame: Arc::clone(&before_frame),
            after_frame: Arc::clone(&after_frame),
            logic_objects: Arc::clone(&logic_objects),
            stop_signal: Arc::clone(&self.stop_signal),
            profiler: Arc::clone(&logic_profiler),
        };
        self.thread_manager
            .spawn("app_logic".to_string(), ThreadPriority::Normal, move || {
                logic(logic_config, events_receiver).expect("TODO: panic message");
            })
            .map_err(|e| ApplicationError::LogicThreadStartError)?;

        // Starting the renderer thread
        let renderer_config = RendererThreadConfig {
            view_config: self.view_config.clone(),
            graphics_config: self.graphics_config.clone(),
            before_frame: Arc::clone(&before_frame),
            after_frame: Arc::clone(&after_frame),
            renderer_objects: Arc::clone(&renderer_objects),
            logic_objects: Arc::clone(&logic_objects),
            stop_signal: Arc::clone(&self.stop_signal),
            profiler: Arc::clone(&renderer_profiler),
        };
        self.thread_manager
            .spawn("app_rend".to_string(), ThreadPriority::Normal, move || {
                renderer(renderer_config, events_sender).expect("TODO: panic message");
            })
            .map_err(|e| ApplicationError::RendererThreadStartError)?;

        #[cfg(debug_assertions)]
        {
            let statistics_config = StatisticsThreadConfig {
                logic_profiler: Arc::clone(&logic_profiler),
                renderer_profiler: Arc::clone(&renderer_profiler),
                stop_signal: Arc::clone(&self.stop_signal),
                logic_objects: Arc::clone(&logic_objects),
                renderer_objects: Arc::clone(&renderer_objects),
            };
            self.thread_manager
                .spawn(
                    "app_stat".to_string(),
                    ThreadPriority::Normal,
                    move || {
                        statistics(statistics_config).expect("TODO: panic message");
                    },
                )
                .map_err(|e| ApplicationError::StatisticsThreadStartError)?;
        }

        Ok(())
    }

    pub fn stop(&self) {
        info!("Stopping application");
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
