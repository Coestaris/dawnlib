use log::{debug, info};

pub mod application;
pub mod event;
pub(crate) mod input;
pub mod object;
pub(crate) mod object_collection;
pub mod view;
pub(crate) mod vulkan;
mod threads;

pub use crate::vulkan::graphics::GraphicsConfig;

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
