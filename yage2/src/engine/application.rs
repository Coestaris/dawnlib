use crate::core::sync::Rendezvous;
use crate::core::time::{current_us, PeriodCounter, TickCounter};
use crate::engine::input::{Event, InputManager};
use crate::engine::object::{DispatchAction, ObjectCtx, ObjectPtr, Renderable};
use crate::engine::object_collection::ObjectsCollection;
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use log::{debug, info, warn};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct ApplicationConfig {
    pub window_config: WindowConfig,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApplicationError<PlatformError> {
    InitError(PlatformError),

    LogicThreadStartError(LogicThreadError),
    LogicThreadStopError(LogicThreadError),
    LogicThreadJoinError(LogicThreadError),

    RendererThreadStartError(RendererThreadError<PlatformError>),
    RendererThreadStopError(RendererThreadError<PlatformError>),
    RendererThreadJoinError(RendererThreadError<PlatformError>),

    StatisticsThreadStartError(StatisticsThreadError),
    StatisticsThreadStopError(StatisticsThreadError),
    StatisticsThreadJoinError(StatisticsThreadError),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum RendererThreadError<PlatformError> {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,

    WindowCreationError(PlatformError),
    WindowTickError(PlatformError),
    GraphicsTickError,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LogicThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    TheadPanic,
    ThreadJoinError,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum StatisticsThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    TheadPanic,
    ThreadJoinError,
}

#[derive(Default)]
struct RendererProfiler {
    fps: TickCounter,
    renderables: TickCounter,
    drawn_triangles: TickCounter,

    before_frame_sync: PeriodCounter,
    window_tick: PeriodCounter,
    graphics_tick: PeriodCounter,
    after_frame_sync: PeriodCounter,
    copy_objects: PeriodCounter,
}

#[derive(Default)]
struct LogicProfiler {
    ups: TickCounter,
    before_frame_sync: PeriodCounter,
    logic_processing: PeriodCounter,
    after_frame_sync: PeriodCounter,
}

/// Shared data between the renderer and logic threads
pub struct SharedData {
    renderer_objects: Mutex<Vec<Renderable>>,
    logic_objects: Mutex<ObjectsCollection>,
    stop_signal: Arc<AtomicBool>,
}

/// Insert passed statement only if the debug feature is enabled
macro_rules! profile {
    ($statement:expr) => {
        #[cfg(debug_assertions)]
        {
            $statement;
        }
    };
    ($statement:expr, $($args:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $statement($($args)*);
        }
    };
}

/// This function creates a new thread that handles rendering operations.
/// It synchronizes with the logic thread using the provided rendezvous points.
/// It processes window events, renders the scene, and updates the renderable objects.
/// It returns a `JoinHandle` that can be used to join the thread later.
fn start_renderer_thread<Win, PlatformError, Graphics>(
    window_factory: Arc<dyn WindowFactory<Win, PlatformError, Graphics>>,
    before_frame: Arc<Rendezvous>,
    after_frame: Arc<Rendezvous>,
    events_sender: Sender<Event>,
    shared_data: Arc<SharedData>,
    profiler: Arc<RendererProfiler>,
) -> Result<
    JoinHandle<Result<(), RendererThreadError<PlatformError>>>,
    RendererThreadError<PlatformError>,
>
where
    Win: Window<PlatformError, Graphics> + 'static,
    PlatformError: std::fmt::Debug + Send + 'static,
    Graphics: crate::engine::graphics::Graphics + 'static,
{
    let factory = window_factory.clone();
    let shared_data = Arc::clone(&shared_data);
    Ok(thread::Builder::new()
        .name("Renderer".into())
        .spawn(move || {
            let mut window = factory
                .create_window(events_sender)
                .map_err(RendererThreadError::WindowCreationError)?;

            info!("Renderer thread started");
            while !shared_data.stop_signal.load(Ordering::Relaxed) {
                profile!(profiler.fps.tick(1));

                // Stage 1: Synchronize with the logic thread
                profile!(profiler.before_frame_sync.start());
                if !before_frame.wait() {
                    warn!("Thread pre-frame synchronization failed, stopping renderer thread");
                    break;
                }
                profile!(profiler.before_frame_sync.end());

                // Stage 2: Process window events and OS-specific stuff
                profile!(profiler.window_tick.start());
                let win_res = window.tick();
                if cfg!(debug_assertions) {
                    if !win_res.map_err(RendererThreadError::WindowTickError)? {
                        warn!("Window tick returned false, stopping renderer thread");
                        break;
                    }
                } else {
                    // Ignore error and hope for the best
                    if let Ok(false) = win_res {
                        warn!("Window tick returned false, stopping renderer thread");
                        break;
                    }
                }
                profile!(profiler.window_tick.end());

                // Stage 3: Render the scene
                profile!(profiler.graphics_tick.start());
                let renderables = shared_data.renderer_objects.lock().unwrap();
                let tick_result = window
                    .get_graphics()
                    .tick(renderables.as_slice())
                    .map_err(|_| RendererThreadError::GraphicsTickError)?;
                drop(renderables);
                profile!(profiler
                    .drawn_triangles
                    .tick(tick_result.drawn_triangles as u32));

                thread::sleep(std::time::Duration::from_millis(16)); /* 60 FPS */
                profile!(profiler.graphics_tick.end());

                // Stage 4: Synchronize with the logic thread
                profile!(profiler.after_frame_sync.start());
                if !after_frame.wait() {
                    warn!("Thread post-frame synchronization failed, stopping renderer thread");
                    break;
                }
                profile!(profiler.after_frame_sync.end());

                // Stage 5: Copy renderable objects data
                profile!(profiler.copy_objects.start());
                let mut objects_collection = shared_data.logic_objects.lock().unwrap();
                match objects_collection.updated_renderables() {
                    Some(updated) => {
                        profile!(profiler.renderables.tick(1));
                        let mut renderables = shared_data.renderer_objects.lock().unwrap();
                        renderables.clear();
                        for val in updated.values() {
                            renderables.push(val.clone());
                        }
                        drop(renderables);
                    }
                    None => {
                        // No renderables updated, nothing to do
                    }
                }
                drop(objects_collection);
                profile!(profiler.copy_objects.end());
            }

            info!("Renderer thread stopping gracefully");

            // Ensure the window is properly closed
            if let Err(e) = window.kill() {
                warn!("Failed to close window: {:?}", e);
            }

            Ok(())
        })
        .map_err(|_| RendererThreadError::ThreadSpawnFailed)?)
}

/// This function creates a new thread that handles the game logic.
/// It synchronizes with the renderer thread using the provided rendezvous points.
/// It processes input events, updates game logic, and dispatches events to the objects.
/// It returns a `JoinHandle` that can be used to join the thread later.
fn start_logic_thread(
    before_frame: Arc<Rendezvous>,
    after_frame: Arc<Rendezvous>,
    events_receiver: Receiver<Event>,
    shared_data: Arc<SharedData>,
    eps: Arc<TickCounter>,
    profiler: Arc<LogicProfiler>,
) -> Result<JoinHandle<Result<(), LogicThreadError>>, LogicThreadError> {
    Ok(thread::Builder::new()
        .name("Logic".into())
        .spawn(move || {
            std::panic::catch_unwind(|| {
                info!("Logic thread started");
                let mut input_manager = InputManager::new(events_receiver, eps);
                let mut first_tick = true;
                let mut prev_tick = 0;
                while !shared_data.stop_signal.load(Ordering::Relaxed) {
                    profile!(profiler.ups.tick(1));

                    // Stage 1: Synchronize with the renderer thread
                    profile!(profiler.before_frame_sync.start());
                    if !before_frame.wait() {
                        warn!("Thread pre-frame synchronization failed, stopping logic thread");
                        break;
                    }
                    profile!(profiler.before_frame_sync.end());

                    // Stage 2: Process logic
                    profile!(profiler.logic_processing.start());
                    let mut objects_collection = shared_data.logic_objects.lock().unwrap();

                    // Stage 2.1: Process input events
                    let mut events = input_manager.poll_events();
                    for event in events.iter() {
                        input_manager.on_event(event);
                    }

                    // Stage 2.2: Dispatch events to objects
                    let current_us = current_us();
                    let time_delta = (current_us - prev_tick) as f32 / 1000.0;
                    prev_tick = current_us;
                    let object_ctx = ObjectCtx {
                        input_manager: &input_manager,
                    };
                    if first_tick {
                        // On the first tick, we need to initialize the objects
                        // and dispatch the Create event to them.
                        events.push(Event::Create);
                        first_tick = false;
                    }
                    events.push(Event::Update(time_delta));
                    for event in events {
                        let actions = objects_collection.dispatch_event(&object_ctx, &event);
                        for action in actions.iter() {
                            match action {
                                DispatchAction::QuitApplication => {
                                    info!("Quit application event received, stopping logic thread");
                                    shared_data.stop_signal.store(true, Ordering::Relaxed);
                                    break;
                                }

                                // The majority of actions are handled in the object collection
                                // and do not require any special handling here.
                                _ => {
                                    // No special handling needed for other actions
                                }
                            }
                        }
                    }
                    // Make sure that mutex is unlocked before the next synchronization point
                    drop(objects_collection);
                    profile!(profiler.logic_processing.end());

                    // Stage 3: Synchronize with the renderer thread
                    profile!(profiler.after_frame_sync.start());
                    if !after_frame.wait() {
                        warn!("Thread post-frame synchronization failed, stopping logic thread");
                        break;
                    }
                    profile!(profiler.after_frame_sync.end());
                }

                info!("Logic thread stopping gracefully");
                Ok(())
            })
            .map_err(|_| {
                warn!("Logic thread panicked, stopping thread");
                shared_data.stop_signal.store(true, Ordering::Relaxed);
                before_frame.unlock();
                after_frame.unlock();
                LogicThreadError::TheadPanic
            })?
        })
        .map_err(|_| LogicThreadError::ThreadSpawnFailed)?)
}

/// This function creates a new thread that handles statistics gathering.
/// It runs only in debug mode and collects various performance metrics
/// such as FPS, UPS, number of renderables, drawn triangles, and event processing speed.
/// It returns a `JoinHandle` that can be used to join the thread later.
#[cfg(debug_assertions)]
fn start_statistics_thread(
    renderer_profiler: Arc<RendererProfiler>,
    logic_profiler: Arc<LogicProfiler>,
    eps: Arc<TickCounter>,
    shared_data: Arc<SharedData>,
) -> Result<JoinHandle<Result<(), StatisticsThreadError>>, StatisticsThreadError> {
    Ok(thread::Builder::new()
        .name("Stat".into())
        .spawn(move || {
            std::panic::catch_unwind(|| {
                info!("Statistics thread started");

                logic_profiler.ups.reset();
                renderer_profiler.fps.reset();
                renderer_profiler.renderables.reset();
                renderer_profiler.drawn_triangles.reset();
                eps.reset();

                let mut reset_counter = 0;
                while !shared_data.stop_signal.load(Ordering::Relaxed) {
                    renderer_profiler.fps.update();
                    renderer_profiler.renderables.update();
                    renderer_profiler.drawn_triangles.update();
                    logic_profiler.ups.update();
                    eps.update();

                    if reset_counter % 10 == 0 {
                        logic_profiler.ups.reset();
                        renderer_profiler.fps.reset();
                        renderer_profiler.renderables.reset();
                        renderer_profiler.drawn_triangles.reset();
                        eps.reset();
                    }
                    reset_counter += 1;

                    let (min_fps, fps, max_fps) = renderer_profiler.fps.get_stat();
                    let (min_ups, ups, max_ups) = logic_profiler.ups.get_stat();
                    let (_, r_renderables_avg, _) =
                        renderer_profiler.renderables.get_stat();

                    let (_, r_bf_avg, _) =
                        renderer_profiler.before_frame_sync.get_stat();
                    let (_, r_wt_avg, _) = renderer_profiler.window_tick.get_stat();
                    let (_, r_gt_avg, _) = renderer_profiler.graphics_tick.get_stat();
                    let (_, r_af_avg, _) =
                        renderer_profiler.after_frame_sync.get_stat();
                    let (_, r_co_avg, _) = renderer_profiler.copy_objects.get_stat();

                    let (_, l_bf_avg, _) =
                        logic_profiler.before_frame_sync.get_stat();
                    let (_, l_lp_avg, _) = logic_profiler.logic_processing.get_stat();
                    let (_, l_af_avg, _) =
                        logic_profiler.after_frame_sync.get_stat();

                    let (_, eps_avg, _) = eps.get_stat();

                    let objects = shared_data.logic_objects.lock().unwrap();
                    let num_objects = objects.alive_objects().len();
                    drop(objects);
                    let renderables = shared_data.renderer_objects.lock().unwrap();
                    let num_renderables = renderables.len();
                    drop(renderables);

                    info!(
                        "Obj: {:}, Ev: {:2.0}, R: {:}, FPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1} {:.1} {:.1} {:.0}], UPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1}]",
                        num_objects,
                        eps_avg,
                        num_renderables,
                        min_fps,
                        fps,
                        max_fps,
                        r_bf_avg,
                        r_wt_avg,
                        r_gt_avg,
                        r_af_avg,
                        r_co_avg,
                        r_renderables_avg,
                        min_ups,
                        ups,
                        max_ups,
                        l_bf_avg,
                        l_lp_avg,
                        l_af_avg,
                    );

                    thread::sleep(std::time::Duration::from_millis(1000));
                }

                info!("Statistics thread stopping gracefully");
                Ok(())
            }).map_err(|_| {
                warn!("Statistics thread panicked, stopping thread");
                shared_data.stop_signal.store(true, Ordering::Relaxed);
                renderer_profiler.fps.reset();
                logic_profiler.ups.reset();
                eps.reset();
                StatisticsThreadError::TheadPanic
            })?
        })
        .map_err(|_| StatisticsThreadError::ThreadSpawnFailed)?)
}

fn log_prelude() {
    let version = env!("CARGO_PKG_VERSION");
    let rust_version = env!("CARGO_PKG_RUST_VERSION");
    let build_timestamp = env!("VERGEN_BUILD_TIMESTAMP");
    let git_sha = env!("VERGEN_GIT_SHA");
    let target_triple = env!("VERGEN_CARGO_TARGET_TRIPLE");
    let os_name = env!("VERGEN_SYSINFO_NAME");
    let os_version = env!("VERGEN_SYSINFO_OS_VERSION");
    let cargo_features = env!("VERGEN_CARGO_FEATURES");
    let profile = if cfg!(debug_assertions) {
        "Debug"
    } else {
        "Release"
    };

    info!("Starting Yage2 Engine");
    debug!(" - Version: {}", version);
    if !rust_version.is_empty() {
        debug!(" - Rust version: {}", rust_version);
    } else {
        debug!(" - Rust version: Unknown");
    }
    debug!(" - Build: {} ({})", build_timestamp, git_sha);
    debug!(
        " - Target: {}. OS: {}, {}",
        target_triple, os_name, os_version
    );
    debug!(" - Profile: {}", profile);
    if !cargo_features.is_empty() {
        debug!(" - Features: {}", cargo_features);
    } else {
        debug!(" - Features: None");
    }
}

pub trait Application<PlatformError, Graphics, Win> {
    fn new(config: ApplicationConfig) -> Result<Self, ApplicationError<PlatformError>>
    where
        Self: Sized;

    fn get_window_factory(
        &self,
    ) -> Arc<dyn WindowFactory<Win, PlatformError, Graphics> + Send + Sync>;

    fn run(&self, objects: Vec<ObjectPtr>) -> Result<(), ApplicationError<PlatformError>>
    where
        Win: Window<PlatformError, Graphics> + 'static,
        PlatformError: std::fmt::Debug + Send + 'static,
        Graphics: crate::engine::graphics::Graphics + 'static,
    {
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
        let factory = self.get_window_factory();

        // Create barriers for synchronization
        let before_frame = Arc::new(Rendezvous::new(2));
        let after_frame = Arc::new(Rendezvous::new(2));

        // Create the renderer profiler used to gather statistics and debug information
        let renderer_profiler = Arc::new(RendererProfiler {
            fps: TickCounter::new(0.3),
            renderables: TickCounter::new(1.0),
            drawn_triangles: TickCounter::new(1.0),
            ..Default::default()
        });
        let logic_profiler = Arc::new(LogicProfiler {
            ups: TickCounter::new(0.3),
            ..Default::default()
        });
        let eps = Arc::new(TickCounter::new(1.0));

        // Create a channel for receiving events from the window handler that is usually handled
        // by the OS or renderer thread and sending them to the logic thread.
        let (sender, receiver): (Sender<Event>, Receiver<Event>) = std::sync::mpsc::channel();

        // Create a shared data structure that will be used to share data between the renderer
        let shared_data = Arc::new(SharedData {
            renderer_objects: Mutex::new(Vec::new()),
            logic_objects: Mutex::new(ObjectsCollection::new(objects)),
            stop_signal: Arc::new(AtomicBool::new(false)),
        });

        info!("Starting renderer thread");
        let logic_thread = start_logic_thread(
            Arc::clone(&before_frame),
            Arc::clone(&after_frame),
            receiver,
            Arc::clone(&shared_data),
            Arc::clone(&eps),
            Arc::clone(&logic_profiler),
        )
        .map_err(ApplicationError::LogicThreadStartError)?;

        info!("Starting logic thread");
        let renderer_thread = start_renderer_thread(
            factory,
            Arc::clone(&before_frame),
            Arc::clone(&after_frame),
            sender,
            Arc::clone(&shared_data),
            Arc::clone(&renderer_profiler),
        )
        .map_err(ApplicationError::RendererThreadStartError)?;

        #[cfg(debug_assertions)]
        info!("Starting statistics thread");
        #[cfg(debug_assertions)]
        let statistics_thread = start_statistics_thread(
            Arc::clone(&renderer_profiler),
            Arc::clone(&logic_profiler),
            Arc::clone(&eps),
            Arc::clone(&shared_data),
        )
        .map_err(ApplicationError::StatisticsThreadStartError)?;

        // Main loop

        // Wait for the threads to finish their work
        if cfg!(debug_assertions) {
            // In case of debug run panic, since we have no way to recover from it.
            renderer_thread.join().unwrap().unwrap();
        } else {
            // In case of a release run, we just ignore the result since we're cleaning up anyway.
            let _ = renderer_thread.join();
        }
        info!("Renderer thread finished");

        // Ask all threads to stop
        shared_data.stop_signal.store(true, Ordering::Relaxed);
        before_frame.unlock();
        after_frame.unlock();

        // Wait for the logic thread to finish its work
        if cfg!(debug_assertions) {
            logic_thread.join().unwrap().unwrap();
        } else {
            let _ = logic_thread.join();
        }
        #[cfg(debug_assertions)]
        if cfg!(debug_assertions) {
            statistics_thread.join().unwrap().unwrap();
        } else {
            let _ = statistics_thread.join();
        }

        info!("Logic thread finished");
        Ok(())
    }
}
