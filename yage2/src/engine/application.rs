use crate::core::sync::Rendezvous;
use crate::core::time::{PeriodCounter, TickCounter};
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use log::{debug, info, warn};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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

pub trait Application<PlatformError, Graphics, Win> {
    fn new(config: ApplicationConfig) -> Result<Self, ApplicationError<PlatformError>>
    where
        Self: Sized;

    fn get_window_factory(
        &self,
    ) -> Arc<dyn WindowFactory<Win, PlatformError, Graphics> + Send + Sync>;

    fn run(&self) -> Result<(), ApplicationError<PlatformError>>
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
        let mut renderer_thread =
            RendererThread::<Win, PlatformError, Graphics>::new(self.get_window_factory());
        let mut logic_thread = LogicThread::new();
        #[cfg(debug_assertions)]
        let mut statistics_thread = StatisticsThread::new();

        /* Create barriers for synchronization */
        let before_frame = Arc::new(Rendezvous::new(2));
        let after_frame = Arc::new(Rendezvous::new(2));

        /* Create the renderer profiler */
        let renderer_profiler = Arc::new(RendererProfiler {
            ..Default::default()
        });
        let logic_profiler = Arc::new(LogicProfiler {
            ..Default::default()
        });

        info!("Starting renderer thread");
        logic_thread
            .start(
                Arc::clone(&before_frame),
                Arc::clone(&after_frame),
                Arc::clone(&logic_profiler),
            )
            .map_err(ApplicationError::LogicThreadStartError)?;
        info!("Starting logic thread");
        renderer_thread
            .start(
                Arc::clone(&before_frame),
                Arc::clone(&after_frame),
                Arc::clone(&renderer_profiler),
            )
            .map_err(ApplicationError::RendererThreadStartError)?;
        #[cfg(debug_assertions)]
        {
            info!("Starting statistics thread");
            statistics_thread
                .start(Arc::clone(&renderer_profiler), Arc::clone(&logic_profiler))
                .map_err(ApplicationError::StatisticsThreadStartError)?;
        }

        /* Main loop */

        /* Wait for the threads to finish their work */
        renderer_thread
            .join()
            .map_err(ApplicationError::RendererThreadJoinError)?;
        info!("Renderer thread finished");

        /* Ask all threads to stop */
        logic_thread
            .stop()
            .map_err(ApplicationError::LogicThreadStopError)?;

        before_frame.unlock();
        after_frame.unlock();

        logic_thread
            .join()
            .map_err(ApplicationError::LogicThreadJoinError)?;
        #[cfg(debug_assertions)]
        {
            statistics_thread
                .stop()
                .map_err(ApplicationError::StatisticsThreadStopError)?;
            statistics_thread
                .join()
                .map_err(ApplicationError::StatisticsThreadJoinError)?;
        }
        info!("Logic thread finished");
        Ok(())
    }
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

struct RendererThread<Win, PlatformError, Graphics> {
    window_factory: Arc<dyn WindowFactory<Win, PlatformError, Graphics>>,
    stop_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), RendererThreadError<PlatformError>>>>,
}

impl<Win, PlatformError, Graphics> RendererThread<Win, PlatformError, Graphics> {
    fn new(
        factory: Arc<dyn WindowFactory<Win, PlatformError, Graphics>>,
    ) -> RendererThread<Win, PlatformError, Graphics>
    where
        Win: Window<PlatformError, Graphics>,
    {
        RendererThread {
            window_factory: factory,
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    fn start(
        &mut self,
        before_frame: Arc<Rendezvous>,
        after_frame: Arc<Rendezvous>,
        profiler: Arc<RendererProfiler>,
    ) -> Result<(), RendererThreadError<PlatformError>>
    where
        Win: Window<PlatformError, Graphics> + 'static,
        PlatformError: std::fmt::Debug + Send + 'static,
        Graphics: crate::engine::graphics::Graphics + 'static,
    {
        if self.thread.is_some() {
            return Err(RendererThreadError::AlreadyRunning);
        }

        let factory = Arc::clone(&self.window_factory);
        let stop_signal = self.stop_signal.clone();

        let handle = thread::Builder::new()
            .name("Renderer".into())
            .spawn(move || {
                let mut window = factory
                    .create_window()
                    .map_err(RendererThreadError::WindowCreationError)?;

                info!("Renderer thread started");
                while !stop_signal.load(Ordering::Relaxed) {
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
                    /* TODO: Implement object copying logic here */
                    thread::sleep(std::time::Duration::from_millis(2));
                    profile!(profiler.copy_objects.end());
                }

                info!("Renderer thread stopping gracefully");

                /* Ensure the window is properly closed */
                if let Err(e) = window.kill() {
                    warn!("Failed to close window: {:?}", e);
                }

                Ok(())
            })
            .map_err(|_| RendererThreadError::ThreadSpawnFailed)?;

        self.thread = Some(handle);
        Ok(())
    }

    fn stop(&self) -> Result<(), RendererThreadError<PlatformError>> {
        self.stop_signal.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn join(&mut self) -> Result<(), RendererThreadError<PlatformError>> {
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| RendererThreadError::ThreadJoinError)??;
        }
        Ok(())
    }
}

struct LogicThread {
    stop_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), LogicThreadError>>>,
}

impl LogicThread {
    fn new() -> LogicThread {
        LogicThread {
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    fn start(
        &mut self,
        before_frame: Arc<Rendezvous>,
        after_frame: Arc<Rendezvous>,
        profiler: Arc<LogicProfiler>,
    ) -> Result<(), LogicThreadError> {
        if self.thread.is_some() {
            return Err(LogicThreadError::AlreadyRunning);
        }

        let stop_signal = self.stop_signal.clone();

        let handle = thread::Builder::new()
            .name("Logic".into())
            .spawn(move || {
                info!("Logic thread started");

                while !stop_signal.load(Ordering::Relaxed) {
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
                    /* TODO: Implement logic processing here */
                    thread::sleep(std::time::Duration::from_millis(2));
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
            .map_err(|_| LogicThreadError::ThreadSpawnFailed)?;

        self.thread = Some(handle);
        Ok(())
    }

    fn stop(&self) -> Result<(), LogicThreadError> {
        self.stop_signal.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn join(&mut self) -> Result<(), LogicThreadError> {
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| LogicThreadError::ThreadJoinError)??;
        }
        Ok(())
    }
}

struct StatisticsThread {
    stop_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), StatisticsThreadError>>>,
}

#[cfg(debug_assertions)]
impl StatisticsThread {
    fn new() -> StatisticsThread {
        StatisticsThread {
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    fn start(
        &mut self,
        renderer_profiler: Arc<RendererProfiler>,
        logic_profiler: Arc<LogicProfiler>,
    ) -> Result<(), StatisticsThreadError> {
        if self.thread.is_some() {
            return Err(StatisticsThreadError::AlreadyRunning);
        }

        let stop_signal = self.stop_signal.clone();

        let handle = thread::Builder::new()
            .name("Statistics".into())
            .spawn(move || {
                info!("Statistics thread started");

                logic_profiler.ups.reset();
                renderer_profiler.fps.reset();

                let mut reset_counter = 0;
                while !stop_signal.load(Ordering::Relaxed) {
                    renderer_profiler.fps.update();
                    logic_profiler.ups.update();

                    if reset_counter % 10 == 0 {
                        logic_profiler.ups.reset();
                        renderer_profiler.fps.reset();
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

                    info!(
                        "FPS: {:.1}/{:.1}/{:.1}, UPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1} {:.1} {:.1}][{:.1} {:.1} {:.1}]",
                        1000.0 / min_fps,
                        1000.0 / fps,
                        1000.0 / max_fps,
                        1000.0 / min_ups,
                        1000.0 / ups,
                        1000.0 / max_ups,
                        r_bf_avg,
                        r_wt_avg,
                        r_gt_avg,
                        r_af_avg,
                        r_co_avg,
                        l_bf_avg,
                        l_lp_avg,
                        l_af_avg,
                    );

                    thread::sleep(std::time::Duration::from_millis(1000));
                }

                info!("Statistics thread stopping gracefully");
                Ok(())
            })
            .map_err(|_| StatisticsThreadError::ThreadSpawnFailed)?;

        self.thread = Some(handle);
        Ok(())
    }

    fn stop(&self) -> Result<(), StatisticsThreadError> {
        self.stop_signal.store(true, Ordering::Relaxed);
        Ok(())
    }

    fn join(&mut self) -> Result<(), StatisticsThreadError> {
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| StatisticsThreadError::ThreadJoinError)??;
        }
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
