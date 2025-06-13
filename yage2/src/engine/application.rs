use crate::core::utils::Rendezvous;
use crate::engine::graphics::Graphics;
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use log::{debug, info, warn};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Barrier, Condvar, Mutex,
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

        /* Create barriers for synchronization */
        let before_frame = Arc::new(Rendezvous::new(2));
        let after_frame = Arc::new(Rendezvous::new(2));

        info!("Starting renderer thread");
        logic_thread
            .start(Arc::clone(&before_frame), Arc::clone(&after_frame))
            .map_err(ApplicationError::LogicThreadStartError)?;
        info!("Starting logic thread");
        renderer_thread
            .start(Arc::clone(&before_frame), Arc::clone(&after_frame))
            .map_err(ApplicationError::RendererThreadStartError)?;

        renderer_thread
            .join()
            .map_err(ApplicationError::RendererThreadJoinError)?;
        info!("Renderer thread finished");

        /* Ask all threads to stop */
        logic_thread
            .stop()
            .map_err(ApplicationError::LogicThreadStopError)?;
        logic_thread
            .join()
            .map_err(ApplicationError::LogicThreadJoinError)?;
        info!("Logic thread finished");
        Ok(())
    }
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
            .name("RendererThread".into())
            .spawn(move || {
                let mut window = factory
                    .create_window()
                    .map_err(RendererThreadError::WindowCreationError)?;

                info!("Renderer thread started");
                while !stop_signal.load(Ordering::Relaxed) {
                    /* 1. Synchronize with the logic thread */
                    if !before_frame.wait(Some(1000)) {
                        warn!("Thread pre-frame synchronization failed, stopping renderer thread");
                        break;

                    }

                    /* 2. Process window events and OS specific stuff */
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

                    /* 3. Render the scene */
                    let graphics_tick = window.get_graphics().tick();
                    if cfg!(debug_assertions) {
                        graphics_tick.map_err(|_| RendererThreadError::GraphicsTickError)?;
                    } else {
                        /* Ignore error and hope for the best */
                        let _ = graphics_tick;
                    }

                    /* 4. Synchronize with the logic thread */
                    if !after_frame.wait(Some(1000)) {
                        warn!("Thread post-frame synchronization failed, stopping renderer thread");
                        break;
                    }

                    /* 5. Copy renderable objects data */
                    /* TODO: Implement copying of renderable objects data here */
                }

                info!("Renderer thread stopping gracefully");
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
    ) -> Result<(), LogicThreadError> {
        if self.thread.is_some() {
            return Err(LogicThreadError::AlreadyRunning);
        }

        let stop_signal = self.stop_signal.clone();

        let handle = thread::Builder::new()
            .name("LogicThread".into())
            .spawn(move || {
                info!("Logic thread started");

                loop {
                    /* 1. Synchronize with the renderer thread */
                    if !before_frame.wait(Some(1000)) {
                        warn!("Thread pre-frame synchronization failed, stopping logic thread");
                        break;
                    }

                    /* 2. Process input, update game logic, etc. */
                    /* TODO: Implement logic processing here */
                    thread::sleep(std::time::Duration::from_millis(2));

                    /* 3. Synchronize with the renderer thread */
                    if !after_frame.wait(Some(1000)) {
                        warn!("Thread post-frame synchronization failed, stopping logic thread");
                        break;
                    }

                    /* 4. Check if we need to stop */
                    if stop_signal.load(Ordering::Relaxed) {
                        break;
                    }
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
