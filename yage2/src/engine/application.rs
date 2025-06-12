use crate::engine::graphics::Graphics;
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use log::{debug, info};
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
pub enum ApplicationError<PlatformError> {
    InitError(PlatformError),

    LogicThreadStartError(LogicThreadError),
    LogicThreadStopError(LogicThreadError),
    LogicThreadJoinError(LogicThreadError),

    RendererThreadStartError(RendererThreadError<PlatformError>),
    RendererThreadStopError(RendererThreadError<PlatformError>),
    RendererThreadJoinError(RendererThreadError<PlatformError>),
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

        info!("Starting renderer thread");
        logic_thread
            .start()
            .map_err(ApplicationError::LogicThreadStartError)?;
        info!("Starting logic thread");
        renderer_thread
            .start()
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

#[derive(Debug)]
enum RendererThreadError<PlatformError> {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,

    WindowCreationError(PlatformError),
    WindowTickError(PlatformError),
    GraphicsTickError,
}

#[derive(Debug)]
enum LogicThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,
}

pub struct RendererThread<Win, PlatformError, Graphics> {
    window_factory: Arc<dyn WindowFactory<Win, PlatformError, Graphics>>,
    stop_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), RendererThreadError<PlatformError>>>>,
}

impl<Win, PlatformError, Graphics> RendererThread<Win, PlatformError, Graphics> {
    pub fn new(
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

    pub fn start(&mut self) -> Result<(), RendererThreadError<PlatformError>>
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
                    /* Process window events and OS specific stuff */
                    let win_res = window.tick();
                    if cfg!(debug_assertions) {
                        if !win_res.map_err(RendererThreadError::WindowTickError)? {
                            debug!("Window tick returned false, stopping renderer thread");
                            break;
                        }
                    } else {
                        /* Ignore error and hope for the best */
                        if let Ok(false) = win_res {
                            debug!("Window tick returned false, stopping renderer thread");
                            break;
                        }
                    }

                    /* Render the scene */
                    let graphics_tick = window.get_graphics().tick();
                    if cfg!(debug_assertions) {
                        graphics_tick.map_err(|_| RendererThreadError::GraphicsTickError)?;
                    } else {
                        /* Ignore error and hope for the best */
                        let _ = graphics_tick;
                    }
                }

                info!("Renderer thread stopping gracefully");
                Ok(())
            })
            .map_err(|_| RendererThreadError::ThreadSpawnFailed)?;

        self.thread = Some(handle);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), RendererThreadError<PlatformError>> {
        self.stop_signal.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn join(&mut self) -> Result<(), RendererThreadError<PlatformError>> {
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| RendererThreadError::ThreadJoinError)??;
        }
        Ok(())
    }
}

pub struct LogicThread {
    stop_signal: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), LogicThreadError>>>,
}

impl LogicThread {
    pub fn new() -> LogicThread {
        LogicThread {
            stop_signal: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub fn start(&mut self) -> Result<(), LogicThreadError> {
        if self.thread.is_some() {
            return Err(LogicThreadError::AlreadyRunning);
        }

        let stop_signal = self.stop_signal.clone();

        let handle = thread::Builder::new()
            .name("LogicThread".into())
            .spawn(move || {
                info!("Logic thread started");

                loop {
                    if stop_signal.load(Ordering::Relaxed) {
                        break;
                    }

                    /* TODO: Implement logic processing here */

                    thread::sleep(std::time::Duration::from_millis(16)); // Simulate ~60 FPS
                }

                info!("Logic thread stopping gracefully");
                Ok(())
            })
            .map_err(|_| LogicThreadError::ThreadSpawnFailed)?;

        self.thread = Some(handle);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), LogicThreadError> {
        self.stop_signal.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub fn join(&mut self) -> Result<(), LogicThreadError> {
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| LogicThreadError::ThreadJoinError)??;
        }
        Ok(())
    }
}

pub fn log_prelude() {
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
