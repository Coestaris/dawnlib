mod notes;

use common::logging::CommonLogger;
use common::profilers::profile_threads;
use common::resources::YARCResourceManagerIO;
use log::info;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use yage2_core::resources::{ResourceManager, ResourceManagerConfig, ResourceType};
use yage2_core::threads::{scoped, ThreadManagerConfig, ThreadPriority};
use yage2_sound::backend::BackendSpecificConfig;
use yage2_sound::entities::bus::Bus;
use yage2_sound::entities::effects::bypass::BypassEffect;
use yage2_sound::entities::sources::waveform::WaveformSource;
use yage2_sound::entities::{NodeRef, Sink};
use yage2_sound::manager::{AudioManager, AudioManagerConfig};

fn profile_audio(frame: &yage2_sound::manager::ProfileFrame) {
    // // Calculate the time in milliseconds, the generator thread
    // // is maximally allowed to take to fill the device buffer.
    // let allowed_time = 1000.0
    //     / ((frame.sample_rate as usize * (frame.device_buffer_size / frame.block_size))
    //         / (frame.device_buffer_size)) as f32;
    //
    // // Calculate the average load of the generator thread
    // let proc_load_precent = frame.gen_av / allowed_time * 100.0;
    //
    // // When no events are processed, we cannot calculate the load
    // // (since the thread is not running).
    // // Assume that events thread has the same maximum allowed time
    // // as the generator thread.
    // let events_load_precent = if frame.events_tps_av == 0.0 {
    //     0.0
    // } else {
    //     frame.events_av / allowed_time * 100.0
    // };
    //
    // // Buffer health is the ratio of available samples in the ring buffer.
    // // Zero means that the buffer is empty and the audio device is starving.
    // // One means that the buffer is full and the audio device has some samples in reserve.
    // let buffer_health_precent = frame.available_av / frame.ring_buffer_size as f32 * 100.0;
    //
    // info!(
    //     "Gen load: {:.1}%. Ev load {:.1}% ({:.0}). Buffer health: {:.1}% ({}/{})",
    //     proc_load_precent,
    //     events_load_precent,
    //     frame.events_tps_av,
    //     buffer_health_precent,
    //     frame.available_min,
    //     frame.available_max
    // );
}

fn main() {
    // Initialize logging
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let resource_manager = Arc::new(ResourceManager::new(ResourceManagerConfig {
        backend: Box::new(YARCResourceManagerIO::new("demo_sound.yarc".to_string())),
    }));
    resource_manager.poll_io().unwrap();

    // TODO: Using leaked boxes for now.
    // This is a temporary solution until we have a proper resource management system.
    let bypass_effect = Box::leak(Box::new(BypassEffect {}));
    let waveform_source = Box::leak(Box::new(WaveformSource::new(None, None, None)));
    let master_bus = Bus::new(1.0, bypass_effect, waveform_source);
    let sink = Sink::new(master_bus);

    let thread_manager_config = ThreadManagerConfig::default();
    let _ = scoped(thread_manager_config, |manager| {
        let audio_manager_config = AudioManagerConfig {
            thread_manager: &manager,
            resource_manager: Arc::clone(&resource_manager),
            backend_specific: BackendSpecificConfig {},
            profiler_handler: Some(profile_audio),
            sample_rate: 48_000,
        };
        let mut audio_manager = AudioManager::new(audio_manager_config, sink)
            .expect("Failed to create audio device");

        sleep(Duration::from_millis(2000));
    });

    resource_manager.finalize_all(ResourceType::AudioWAV);

    info!("Yage2 Engine finished");
}
