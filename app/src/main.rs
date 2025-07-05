mod objects;

extern crate core;

use crate::objects::Point;
use ansi_term::Colour::{Blue, Cyan, Green, Red, Yellow};
use log::{info, Level, LevelFilter, Metadata, Record};
use std::collections::HashMap;
use std::fs::read;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::sleep;
use yage2_app::create_object;
use yage2_app::engine::application::Application;
use yage2_core::resources::{
    ResourceChecksum, ResourceHeader, ResourceManager, ResourceManagerConfig, ResourceManagerIO,
    ResourceType,
};
use yage2_core::threads::{ThreadManager, ThreadManagerConfig};
use yage2_core::utils::format_now;
use yage2_sound::backend::BackendSpecificConfig;
use yage2_sound::control::{AudioController, Controller};
use yage2_sound::dsp::bus::Bus;
use yage2_sound::dsp::sources::clip::{ClipMessage, ClipSource};
use yage2_sound::dsp::sources::group::GroupSource;
use yage2_sound::dsp::sources::waveform::{WaveformMessage, WaveformSource};
use yage2_sound::dsp::SourceType;
use yage2_sound::manager::{AudioManager, AudioManagerConfig};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            fn colored_level(level: Level) -> ansi_term::Colour {
                match level {
                    Level::Error => Red,
                    Level::Warn => Yellow,
                    Level::Info => Green,
                    Level::Debug => Blue,
                    Level::Trace => Cyan,
                }
            }

            let formatted_date = format_now().unwrap_or("unknown".to_string());

            println!(
                "[{}][{:>19}][{:>14}]: {} [{}:{}]",
                Cyan.paint(formatted_date),
                Yellow
                    .paint(std::thread::current().name().unwrap_or("main"))
                    .to_string(),
                colored_level(record.level())
                    .paint(record.level().to_string())
                    .to_string(),
                record.args(),
                Green.paint(record.file().unwrap_or("unknown")),
                Green.paint(record.line().unwrap_or(0).to_string())
            );
        }
    }

    fn flush(&self) {}
}

enum NoteName {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

fn get_current_exe() -> std::path::PathBuf {
    std::env::current_exe().expect("Failed to get current executable path")
}

struct Note {
    name: NoteName,
    octave: u8,
}

impl Note {
    fn new(name: NoteName, octave: u8) -> Self {
        if octave < 0 || octave > 8 {
            panic!("Octave must be between 0 and 8");
        }
        Note { name, octave }
    }

    fn frequency(&self) -> f32 {
        let base_frequency = match self.name {
            NoteName::C => 261.63,
            NoteName::CSharp => 277.18,
            NoteName::D => 293.66,
            NoteName::DSharp => 311.13,
            NoteName::E => 329.63,
            NoteName::F => 349.23,
            NoteName::FSharp => 369.99,
            NoteName::G => 392.00,
            NoteName::GSharp => 415.30,
            NoteName::A => 440.00,
            NoteName::ASharp => 466.16,
            NoteName::B => 493.88,
        };
        base_frequency * (2f32).powi(self.octave as i32 - 4)
    }
}

struct ResourcesIO {
    containers: HashMap<String, yage2_yarc::Container>,
}

impl ResourcesIO {
    fn new() -> ResourcesIO {
        ResourcesIO {
            containers: HashMap::new(),
        }
    }
}

impl ResourceManagerIO for ResourcesIO {
    fn has_updates(&self) -> bool {
        true
    }

    fn enumerate_resources(&mut self) -> Result<HashMap<String, ResourceHeader>, String> {
        self.containers =
            yage2_yarc::read(get_current_exe().parent().unwrap().join("resources.yarc"))
                .map_err(|e| format!("Failed to read resources: {}", e.to_string()))?;

        info!("Loaded {} resources", self.containers.len());
        for (name, container) in &self.containers {
            info!(
                "Resource: {} (type {:?}). Size: {} bytes)",
                name,
                container.metadata.header.resource_type,
                container.binary.len()
            );
        }

        let mut result = HashMap::new();
        for (name, container) in &self.containers {
            result.insert(name.clone(), container.metadata.header.clone());
        }

        Ok(result)
    }

    fn load(&mut self, name: String) -> Result<Vec<u8>, String> {
        if let Some(container) = self.containers.get(&name) {
            info!("Loading resource: {}", name);
            // TODO: get rid of clone
            Ok(container.binary.clone())
        } else {
            Err(format!("Resource {} not found", name))
        }
    }
}

fn profile_threads(frame: &yage2_core::threads::ProfileFrame) {
    let mut str = String::with_capacity(1024);
    for thread in &frame.threads {
        str.push_str(&format!(
            "{}: ({:.1}) ",
            thread.name,
            thread.cpu_utilization * 100.0
        ));
    }
    info!("{}", str);
}

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
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap();
    //
    // let application_config = yage2::engine::application::ApplicationConfig {
    //     window_config: yage2::engine::window::WindowConfig {
    //         title: "Yage2 Engine".to_string(),
    //         width: 1280,
    //         height: 720,
    //     },
    // };
    //
    // let app = yage2::create_app!(application_config).unwrap();
    // let objects = vec![
    //     create_object!(objects::EventListener),
    //     create_object!(objects::SimpleObject::new(Point::new(10.0, 200.0))),
    //     create_object!(objects::SimpleObject::new(Point::new(100.0, 200.0))),
    //     create_object!(objects::SimpleObject::new(Point::new(200.0, 200.0))),
    // ];
    // app.run(objects).unwrap();

    let resource_manager = Arc::new(ResourceManager::new(ResourceManagerConfig {
        backend: Box::new(ResourcesIO::new()),
    }));
    resource_manager.poll_io().unwrap();

    let thread_manager: Arc<ThreadManager> = Arc::new(ThreadManager::new(ThreadManagerConfig {
        profile_handle: Some(profile_threads),
    }));

    let (clip_source, clip_control) = ClipSource::new();
    let (master_bus, _) = Bus::new()
        .set_source(SourceType::Clip(clip_source))
        .set_volume(0.5)
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

    audio_controller.send_and_notify(&clip_control, ClipMessage::Play(clip.clone()));

    sleep(std::time::Duration::from_millis(15000));

    audio_manager.stop().unwrap();

    thread_manager.join_all();

    resource_manager.finalize_all(ResourceType::AudioWAV);
    info!("Yage2 Engine finished");
}
