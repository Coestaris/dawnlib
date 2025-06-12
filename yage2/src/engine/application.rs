use crate::engine::window::Window;
use log::{debug, info};

#[derive(Debug)]
pub enum ApplicationError<PlatformError> {
    InitError(PlatformError),

    LogicThreadStartError(LogicThreadError),
    LogicThreadStopError(LogicThreadError),
    LogicThreadJoinError(LogicThreadError),

    StatisticsThreadStartError(StatisticsThreadError),
    StatisticsThreadStopError(StatisticsThreadError),
    StatisticsThreadJoinError(StatisticsThreadError),

    ResourcesThreadStartError(ResourcesThreadError),
    ResourcesThreadStopError(ResourcesThreadError),
    ResourcesThreadJoinError(ResourcesThreadError),

    RendererThreadStartError(RendererThreadError),
    RendererThreadStopError(RendererThreadError),
    RendererThreadJoinError(RendererThreadError),
}

pub trait Application {
    type Win;
    type PlatformError;

    fn new(title: &str, width: u32, height: u32) -> Result<Self, ApplicationError<Self::PlatformError>>
    where
        Self::Win: Window<Error = Self::PlatformError>,
        Self: Sized;

    fn run(&self) -> Result<(), ApplicationError<Self::PlatformError>> {
        log_prelude();

        /*
         * Threading model:
         *           Sync                                     Sync
         * Renderer:   ||  Start drawing scene   |              || Copy objects data
         * Logic:      ||  Process input | Update objects | etc ||
         */
        let renderer_thread = RendererThread {};
        let statistics_thread = StatisticsThread {};
        let resources_thread = ResourcesThread {};
        let logic_thread = LogicThread {};

        info!("Starting renderer thread");
        logic_thread
            .start()
            .map_err(ApplicationError::LogicThreadStartError)?;
        info!("Starting logic thread");
        renderer_thread
            .start()
            .map_err(ApplicationError::RendererThreadStartError)?;
        info!("Starting statistics thread");
        statistics_thread
            .start()
            .map_err(ApplicationError::StatisticsThreadStartError)?;
        info!("Starting resources thread");
        resources_thread
            .start()
            .map_err(ApplicationError::ResourcesThreadStartError)?;

        renderer_thread
            .join()
            .map_err(ApplicationError::RendererThreadJoinError)?;
        info!("Renderer thread finished");

        /* Ask all threads to stop */
        logic_thread
            .stop()
            .map_err(ApplicationError::LogicThreadStopError)?;
        statistics_thread
            .stop()
            .map_err(ApplicationError::StatisticsThreadStopError)?;
        resources_thread
            .stop()
            .map_err(ApplicationError::ResourcesThreadStopError)?;

        logic_thread
            .join()
            .map_err(ApplicationError::LogicThreadJoinError)?;
        info!("Logic thread finished");
        statistics_thread
            .join()
            .map_err(ApplicationError::StatisticsThreadJoinError)?;
        info!("Statistics thread finished");
        resources_thread
            .join()
            .map_err(ApplicationError::ResourcesThreadJoinError)?;
        info!("Resources thread finished");

        info!("Yage2 Engine finished");
        Ok(())
    }
}

#[derive(Debug)]
enum RendererThreadError {}
#[derive(Debug)]
enum LogicThreadError {}
#[derive(Debug)]
enum StatisticsThreadError {}
#[derive(Debug)]
enum ResourcesThreadError {}

trait ThreadTrait {
    type Error;

    fn start(&self) -> Result<(), Self::Error>;
    fn stop(&self) -> Result<(), Self::Error>;
    fn join(&self) -> Result<(), Self::Error>;
}


struct RendererThread {
    thread: std::thread::Thread
}



impl ThreadTrait for RendererThread {
    type Error = RendererThreadError;
    fn start(&self) -> Result<(), RendererThreadError> {
        // Start the renderer thread
        Ok(())
    }

    fn stop(&self) -> Result<(), RendererThreadError> {
        // Stop the renderer thread
        Ok(())
    }

    fn join(&self) -> Result<(), RendererThreadError> {
        // Join the renderer thread
        Ok(())
    }
}

struct LogicThread {
    // Logic thread implementation
}

struct StatisticsThread {
    // Statistics thread implementation
}

struct ResourcesThread {
    // Resources thread implementation
}

impl ThreadTrait for LogicThread {
    type Error = LogicThreadError;
    fn start(&self) -> Result<(), LogicThreadError> {
        // Start the logic thread
        Ok(())
    }

    fn stop(&self) -> Result<(), LogicThreadError> {
        // Stop the logic thread
        Ok(())
    }

    fn join(&self) -> Result<(), LogicThreadError> {
        // Join the logic thread
        Ok(())
    }
}

impl ThreadTrait for StatisticsThread {
    type Error = StatisticsThreadError;
    fn start(&self) -> Result<(), StatisticsThreadError> {
        // Start the statistics thread
        Ok(())
    }

    fn stop(&self) -> Result<(), StatisticsThreadError> {
        // Stop the statistics thread
        Ok(())
    }

    fn join(&self) -> Result<(), StatisticsThreadError> {
        // Join the statistics thread
        Ok(())
    }
}

impl ThreadTrait for ResourcesThread {
    type Error = ResourcesThreadError;

    fn start(&self) -> Result<(), ResourcesThreadError> {
        // Start the resources thread
        Ok(())
    }

    fn stop(&self) -> Result<(), ResourcesThreadError> {
        // Stop the resources thread
        Ok(())
    }

    fn join(&self) -> Result<(), ResourcesThreadError> {
        // Join the resources thread
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
