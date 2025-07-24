mod notes;

use crate::notes::{Note, NoteName};
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
use yage2_sound::entities::sinks::InterleavedSink;
use yage2_sound::entities::sources::multiplexer::Multiplexer3Source;
use yage2_sound::entities::sources::waveform::WaveformSource;
use yage2_sound::manager::{AudioManager, AudioManagerConfig};

fn profile_audio(frame: &yage2_sound::manager::ProfileFrame) {
    // Calculate the time in milliseconds, the renderer thread
    // is maximally allowed to take to fill the device buffer.
    let allowed_time = (1000.0 / frame.sample_rate as f32) * frame.block_size as f32;

    // When no events are processed, we cannot calculate the load
    // (since the thread is not running).
    // Assume that events thread has the same maximum allowed time
    // as the renderer thread.
    let events_load_precent = if frame.events_tps_av == 0.0 {
        0.0
    } else {
        frame.events_av / allowed_time * 100.0
    };

    info!(
        "T: {:.0}. Render: {:.1}ms ({:.1}%). Ev {:.1}ms ({:.1}%) ({:.0})",
        frame.render_tps_av,
        frame.render_av,
        frame.render_av / allowed_time * 100.0,
        frame.events_av,
        events_load_precent,
        frame.events_tps_av,
    );
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
    let waveform_source1 = Box::leak(Box::new(WaveformSource::new(
        None,
        Some(Note::new(NoteName::C, 3).frequency()),
        None,
    )));
    let waveform_source2 = Box::leak(Box::new(WaveformSource::new(
        None,
        Some(Note::new(NoteName::E, 4).frequency()),
        None,
    )));
    let waveform_source3 = Box::leak(Box::new(WaveformSource::new(
        None,
        Some(Note::new(NoteName::B, 4).frequency()),
        None,
    )));
    let multiplexer = Multiplexer3Source::new(
        waveform_source1,
        waveform_source2,
        waveform_source3,
        0.3,
        0.3,
        0.3,
    );
    // let master_bus = Bus::new(bypass_effect, multiplexer);
    let sink = InterleavedSink::new(multiplexer, 48_000);

    let thread_manager_config = ThreadManagerConfig::default();
    let _ = scoped(thread_manager_config, |manager| {
        let audio_manager_config = AudioManagerConfig {
            thread_manager: &manager,
            resource_manager: Arc::clone(&resource_manager),
            backend_specific: BackendSpecificConfig {},
            profiler_handler: Some(profile_audio),
            sample_rate: 48_000,
        };
        let mut audio_manager =
            AudioManager::new(audio_manager_config, sink).expect("Failed to create audio device");

        sleep(Duration::from_millis(10000));
    });

    resource_manager.finalize_all(ResourceType::AudioWAV);

    info!("Yage2 Engine finished");
}
