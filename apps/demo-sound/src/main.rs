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
    info!(
        "Gen: {:.1}/{:.1} (of {:.1}) ({:.0}). Ev: {:.1} ({:.0}). Buffer: {:}/{:}/{:}",
        frame.gen_av,
        frame.write_bulk_av,
        1000.0
            / ((frame.sample_rate as usize * (frame.buffer_size / frame.block_size))
                / (frame.buffer_size)) as f32,
        frame.gen_tps_av,
        frame.events_av,
        frame.events_tps_av,
        frame.available_min,
        frame.available_av,
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
