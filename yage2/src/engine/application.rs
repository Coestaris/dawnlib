use crate::core::sync::Rendezvous;
use crate::core::time::{PeriodCounter, TickCounter};
use crate::engine::app_ctx::ApplicationCtx;
use crate::engine::input::{InputEvent, InputManager};
use crate::engine::object::{Object, Renderable};
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
enum RendererThreadError<PlatformError> {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,

    WindowCreationError(PlatformError),
    WindowTickError(PlatformError),
    GraphicsTickError,
}

#[derive(Debug)]
#[allow(dead_code)]
enum LogicThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,
}

#[derive(Debug)]
#[allow(dead_code)]
enum StatisticsThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,
}

#[derive(Default)]
struct RendererProfiler {
    fps: TickCounter,
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

/* This struct is used to store data that is shared between the
 * logic and renderer threads. */
pub struct SharedData {
    renderer_objects: Mutex<Vec<Renderable>>,
    logic_objects: Mutex<Vec<Box<dyn Object + Send + Sync>>>,
    stop_signal: Arc<AtomicBool>,
}

/* Insert passed statement only if the debug feature is enabled */
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

fn start_renderer_thread<Win, PlatformError, Graphics>(
    window_factory: Arc<dyn WindowFactory<Win, PlatformError, Graphics>>,
    before_frame: Arc<Rendezvous>,
    after_frame: Arc<Rendezvous>,
    events_sender: Sender<InputEvent>,
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
                profile!(profiler.fps.tick());

                /* 1. Synchronize with the logic thread */
                profile!(profiler.before_frame_sync.start());
                if !before_frame.wait() {
                    warn!("Thread pre-frame synchronization failed, stopping renderer thread");
                    break;
                }
                profile!(profiler.before_frame_sync.end());

                /* 2. Process window events and OS specific stuff */
                profile!(profiler.window_tick.start());
                let win_res = window.tick();
                if cfg!(debug_assertions) {
                    if !win_res.map_err(RendererThreadError::WindowTickError)? {
                        warn!("Window tick returned false, stopping renderer thread");
                        break;
                    }
                } else {
                    /* Ignore error and hope for the best */
                    if let Ok(false) = win_res {
                        warn!("Window tick returned false, stopping renderer thread");
                        break;
                    }
                }
                profile!(profiler.window_tick.end());

                /* 3. Render the scene */
                profile!(profiler.graphics_tick.start());
                let graphics_tick = window.get_graphics().tick();
                if cfg!(debug_assertions) {
                    graphics_tick.map_err(|_| RendererThreadError::GraphicsTickError)?;
                } else {
                    /* Ignore error and hope for the best */
                    let _ = graphics_tick;
                }
                thread::sleep(std::time::Duration::from_millis(16)); /* 60 FPS */
                profile!(profiler.graphics_tick.end());

                /* 4. Synchronize with the logic thread */
                profile!(profiler.after_frame_sync.start());
                if !after_frame.wait() {
                    warn!("Thread post-frame synchronization failed, stopping renderer thread");
                    break;
                }
                profile!(profiler.after_frame_sync.end());

                /* 5. Copy renderable objects data */
                profile!(profiler.copy_objects.start());
                let mut renderables = shared_data.renderer_objects.lock().unwrap();
                let mut objects = shared_data.logic_objects.lock().unwrap();
                renderables.clear();
                for object in objects.iter_mut() {
                    if let Some(renderable) = object.renderable() {
                        renderables.push(renderable.clone());
                    }
                }
                drop(objects);
                drop(renderables);
                profile!(profiler.copy_objects.end());
            }

            info!("Renderer thread stopping gracefully");

            /* Ensure the window is properly closed */
            if let Err(e) = window.kill() {
                warn!("Failed to close window: {:?}", e);
            }

            Ok(())
        })
        .map_err(|_| RendererThreadError::ThreadSpawnFailed)?)
}

fn start_logic_thread(
    before_frame: Arc<Rendezvous>,
    after_frame: Arc<Rendezvous>,
    events_receiver: Receiver<InputEvent>,
    shared_data: Arc<SharedData>,
    eps: Arc<TickCounter>,
    profiler: Arc<LogicProfiler>,
) -> Result<JoinHandle<Result<(), LogicThreadError>>, LogicThreadError> {
    Ok(thread::Builder::new()
        .name("Logic".into())
        .spawn(move || {
            info!("Logic thread started");

            let mut ctx = ApplicationCtx {
                input_manager: InputManager::new(events_receiver, eps),
            };

            while !shared_data.stop_signal.load(Ordering::Relaxed) {
                profile!(profiler.ups.tick());

                /* 1. Synchronize with the renderer thread */
                profile!(profiler.before_frame_sync.start());
                if !before_frame.wait() {
                    warn!("Thread pre-frame synchronization failed, stopping logic thread");
                    break;
                }
                profile!(profiler.before_frame_sync.end());

                /* 2. Process input, update game logic, etc. */
                profile!(profiler.logic_processing.start());
                let mut objects = shared_data.logic_objects.lock().unwrap();

                /* Process input events */
                ctx.input_manager.poll_events();
                ctx.input_manager.update();
                ctx.input_manager.dispatch_events(&ctx, &mut objects);

                /* Call on_tick for each game object */
                for object in objects.iter_mut() {
                    object.on_tick(&ctx);
                }

                /* Make sure that mutex is unlocked
                 * before the next synchronization point */
                drop(objects);
                profile!(profiler.logic_processing.end());

                /* 3. Synchronize with the renderer thread */
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
        .map_err(|_| LogicThreadError::ThreadSpawnFailed)?)
}

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
            info!("Statistics thread started");

            logic_profiler.ups.reset();
            renderer_profiler.fps.reset();
            eps.reset();

            let mut reset_counter = 0;
            while !shared_data.stop_signal.load(Ordering::Relaxed) {
                renderer_profiler.fps.update();
                logic_profiler.ups.update();
                eps.update();

                if reset_counter % 10 == 0 {
                    logic_profiler.ups.reset();
                    renderer_profiler.fps.reset();
                    eps.reset();
                }
                reset_counter += 1;

                let (min_fps, fps, max_fps) = renderer_profiler.fps.get_stat();
                let (min_ups, ups, max_ups) = logic_profiler.ups.get_stat();

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
                let num_objects = objects.len();
                drop(objects);

                info!(
                    "Obj: {:}, Ev: {:2.0}, FPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1} {:.1} {:.1}], UPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1}]",
                    num_objects,
                    eps_avg,
                    min_fps,
                    fps,
                    max_fps,
                    r_bf_avg,
                    r_wt_avg,
                    r_gt_avg,
                    r_af_avg,
                    r_co_avg,
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
        })
        .map_err(|_| StatisticsThreadError::ThreadSpawnFailed)?)
}

pub trait Application<PlatformError, Graphics, Win> {
    fn new(config: ApplicationConfig) -> Result<Self, ApplicationError<PlatformError>>
    where
        Self: Sized;

    fn get_window_factory(
        &self,
    ) -> Arc<dyn WindowFactory<Win, PlatformError, Graphics> + Send + Sync>;

    fn run(
        &self,
        objects: Vec<Box<dyn Object + Send + Sync>>,
    ) -> Result<(), ApplicationError<PlatformError>>
    where
        Win: Window<PlatformError, Graphics> + 'static,
        PlatformError: std::fmt::Debug + Send + 'static,
        Graphics: crate::engine::graphics::Graphics + 'static,
    {
        log_prelude();

        /*
         * Threading model:
         *           Sync                                     Sync
         * Renderer:   ||  Start drawing scene   |              || Copy objects data
         * Logic:      ||  Process input | Update objects | etc ||
         */
        let factory = self.get_window_factory();

        /* Create barriers for synchronization */
        let before_frame = Arc::new(Rendezvous::new(2));
        let after_frame = Arc::new(Rendezvous::new(2));

        /* Create the renderer profiler */
        let renderer_profiler = Arc::new(RendererProfiler {
            fps: TickCounter::new(0.3),
            ..Default::default()
        });
        let logic_profiler = Arc::new(LogicProfiler {
            ups: TickCounter::new(0.3),
            ..Default::default()
        });
        let eps = Arc::new(TickCounter::new(1.0));

        /* Create the application context */
        let (sender, receiver): (Sender<InputEvent>, Receiver<InputEvent>) =
            std::sync::mpsc::channel();

        /* Create the shared data */
        let shared_data = Arc::new(SharedData {
            renderer_objects: Mutex::new(Vec::new()),
            logic_objects: Mutex::new(objects),
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

        /* Main loop */

        /* Wait for the threads to finish their work */
        /* Just panic. There's nothing we can do in case of an error */
        renderer_thread.join().unwrap().unwrap();
        info!("Renderer thread finished");

        /* Ask all threads to stop */
        shared_data.stop_signal.store(true, Ordering::Relaxed);
        before_frame.unlock();
        after_frame.unlock();

        /* Join all threads */
        logic_thread.join().unwrap().unwrap();
        #[cfg(debug_assertions)]
        statistics_thread.join().unwrap().unwrap();

        info!("Logic thread finished");
        Ok(())
    }
}

fn log_prelude() {
    info!("Starting Yage2 Engine");
    // debug!(" - Version: {} (rust {})", env!("VERGEN_CARGO_PKG_VERSION"), env!("CARGO_PKG_RUST_VERSION"));
    debug!(
        " - Build: {} ({})",
        env!("VERGEN_BUILD_TIMESTAMP"),
        env!("VERGEN_GIT_SHA")
    );
    debug!(
        " - Target: {}, {} {}",
        env!("VERGEN_CARGO_TARGET_TRIPLE"),
        env!("VERGEN_SYSINFO_NAME"),
        env!("VERGEN_SYSINFO_OS_VERSION")
    );
    debug!(" - Features: {}", env!("VERGEN_CARGO_FEATURES"));
}
