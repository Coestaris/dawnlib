use log::{debug, info};

pub mod core;
pub mod engine;

pub mod platforms;

pub fn log_prelude() {
    info!("Starting Yage2 Engine");
    // debug!(" - Version: {} (rust {})", env!("VERGEN_CARGO_PKG_VERSION"), env!("CARGO_PKG_RUST_VERSION"));
    debug!(" - Build: {} ({})", env!("VERGEN_BUILD_TIMESTAMP"), env!("VERGEN_GIT_SHA"));
    debug!(" - Target: {}, {} {}", env!("VERGEN_CARGO_TARGET_TRIPLE"), env!("VERGEN_SYSINFO_NAME"), env!("VERGEN_SYSINFO_OS_VERSION"));
    debug!(" - Features: {}", env!("VERGEN_CARGO_FEATURES"));
}