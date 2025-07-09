mod notes;

use common::logging::CommonLogger;
use common::profilers::profile_threads;
use common::resources::YARCResourceManagerIO;
use log::info;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use yage2_core::resources::{ResourceManager, ResourceManagerConfig, ResourceType};
use yage2_core::threads::{ThreadManager, ThreadManagerConfig};
use yage2_sound::backend::BackendSpecificConfig;
use yage2_sound::control::AudioController;
use yage2_sound::dsp::bus::Bus;
use yage2_sound::dsp::processors::reverb::LineReverbMessage;
use yage2_sound::dsp::sources::sampler::{SamplerMessage, SamplerSource};
use yage2_sound::manager::{AudioManager, AudioManagerConfig};

fn profile_audio(frame: &yage2_sound::manager::ProfileFrame) {
    // Calculate the time in milliseconds, the generator thread
    // is maximally allowed to take to fill the device buffer.
    let allowed_time = 1000.0
        / ((frame.sample_rate as usize * (frame.device_buffer_size / frame.block_size))
            / (frame.device_buffer_size)) as f32;

    // Calculate the average load of the generator thread
    let proc_load_precent = frame.gen_av / allowed_time * 100.0;

    // When no events are processed, we cannot calculate the load
    // (since the thread is not running).
    // Assume that events thread has the same maximum allowed time
    // as the generator thread.
    let events_load_precent = if frame.events_tps_av == 0.0 {
        0.0
    } else {
        frame.events_av / allowed_time * 100.0
    };

    // Buffer health is the ratio of available samples in the ring buffer.
    // Zero means that the buffer is empty and the audio device is starving.
    // One means that the buffer is full and the audio device has some samples in reserve.
    let buffer_health_precent = frame.available_av / frame.ring_buffer_size as f32 * 100.0;

    info!(
        "Gen load: {:.1}%. Ev load {:.1}% ({:.0}). Buffer health: {:.1}% ({}/{})",
        proc_load_precent,
        events_load_precent,
        frame.events_tps_av,
        buffer_health_precent,
        frame.available_min,
        frame.available_max
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

    let thread_manager: Arc<ThreadManager> = Arc::new(ThreadManager::new(ThreadManagerConfig {
        profile_handle: Some(profile_threads),
    }));

    let (reverb, line_reverb_control) = yage2_sound::dsp::processors::reverb::Reverb::new();
    let (sample_source, sampler_control) = SamplerSource::new();
    let (master_bus, _) = Bus::new()
        .set_source(sample_source)
        .set_volume(0.5)
        .add_processor(reverb)
        .set_pan(0.0)
        .build();

    let audio_controller = Arc::new(AudioController::new());
    let audio_manager_config = AudioManagerConfig {
        thread_manager: Arc::clone(&thread_manager),
        resource_manager: Arc::clone(&resource_manager),

        backend_specific: BackendSpecificConfig {},
        master_bus: Arc::new(Mutex::new(master_bus)),
        profiler_handler: Some(profile_audio),
        controller: Arc::clone(&audio_controller),
        sample_rate: 48_000,
    };

    let mut audio_manager =
        AudioManager::new(audio_manager_config).expect("Failed to create audio device");

    let clip = resource_manager.get_resource("loop").unwrap();

    audio_manager.start().unwrap();

    audio_controller.send_and_notify(
        &sampler_control,
        SamplerMessage::Play {
            clip: clip.clone(),
            volume: 0.5,
            pan: 0.0,
        },
    );

    sleep(Duration::from_millis(1000));

    audio_controller.send_and_notify(&sampler_control, SamplerMessage::StopAll);

    sleep(Duration::from_millis(3000));

    audio_controller.send(
        &line_reverb_control,
        LineReverbMessage::SetLineSize(0, 3000),
    );
    audio_controller.send(&line_reverb_control, LineReverbMessage::SetLineFade(0, 0.9));
    audio_controller.send(&line_reverb_control, LineReverbMessage::SetWetLevel(0, 0.9));

    audio_controller.send(
        &line_reverb_control,
        LineReverbMessage::SetLineSize(1, 10000),
    );
    audio_controller.send(&line_reverb_control, LineReverbMessage::SetLineFade(1, 0.9));
    audio_controller.send(&line_reverb_control, LineReverbMessage::SetWetLevel(1, 0.9));

    audio_controller.send(
        &sampler_control,
        SamplerMessage::Play {
            clip: clip.clone(),
            volume: 0.5,
            pan: 0.0,
        },
    );
    audio_controller.notify();

    sleep(Duration::from_millis(1000));
    audio_controller.send_and_notify(&sampler_control, SamplerMessage::StopAll);

    sleep(Duration::from_millis(8000));

    audio_manager.stop().unwrap();

    thread_manager.join_all();

    resource_manager.finalize_all(ResourceType::AudioWAV);
    info!("Yage2 Engine finished");
}
