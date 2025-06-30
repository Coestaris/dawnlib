mod objects;

extern crate core;

use crate::objects::Point;
use ansi_term::Colour::{Blue, Cyan, Green, Red, Yellow};
use log::{info, Level, LevelFilter, Metadata, Record};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use yage2_app::create_object;
use yage2_app::engine::application::Application;
use yage2_core::utils::format_now;
use yage2_sound::device::{Device, DeviceConfig};
use yage2_sound::dsp::bus::Bus;
use yage2_sound::dsp::sources::group::GroupSource;
use yage2_sound::dsp::sources::waveform::{WaveformMessage, WaveformSource};
use yage2_sound::dsp::SourceType;
use yage2_sound::{BackendSpecificConfig, SampleType};

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
                "[{}][{:>17}][{:>14}]: {} [{}:{}]",
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

    let update_bus = Arc::new((Mutex::new(0u8), Condvar::new()));
    let (source1, source1_controller) = WaveformSource::new().build();
    let (bus1, _) = Bus::new()
        .set_source(SourceType::Waveform(source1))
        .set_volume(0.2)
        .build();

    let (source2, source2_controller) = WaveformSource::new().build();
    let (bus2, _) = Bus::new()
        .set_source(SourceType::Waveform(source2))
        .set_volume(0.3)
        .build();

    let (source3, source3_controller) = WaveformSource::new().build();
    let (bus3, _) = Bus::new()
        .set_source(SourceType::Waveform(source3))
        .set_volume(0.4)
        .build();

    let (group, _) = GroupSource::new(vec![bus1, bus2, bus3]);
    let (master_bus, _) = Bus::new()
        .set_source(SourceType::Group(group))
        .set_volume(0.5)
        .set_pan(0.0)
        .build();

    let device_config = DeviceConfig {
        backend_specific: BackendSpecificConfig {},
        main_bus: Arc::new(Mutex::new(master_bus)),
        update_bus: Arc::clone(&update_bus),
        sample_rate: 48_000,
    };

    fn control_waveform(controller: &Sender<WaveformMessage>, note: Note) {
        controller
            .send(WaveformMessage::SetFrequency(note.frequency()))
            .expect("Failed to send frequency control");
    }

    fn notify_bus(update_bus: &(Mutex<u8>, Condvar)) {
        let (lock, cvar) = update_bus;
        let mut update = lock.lock().unwrap();
        *update += 1;
        cvar.notify_all();
    }

    let mut device = Device::new(device_config).expect("Failed to create audio device");
    device.open().unwrap();

    for i in 0..3 {
        control_waveform(&source1_controller, Note::new(NoteName::C, 4));
        control_waveform(&source2_controller, Note::new(NoteName::G, 4));
        control_waveform(&source3_controller, Note::new(NoteName::E, 4));
        notify_bus(&update_bus);

        std::thread::sleep(std::time::Duration::from_secs(2));

        control_waveform(&source1_controller, Note::new(NoteName::C, 4));
        control_waveform(&source2_controller, Note::new(NoteName::GSharp, 4));
        control_waveform(&source3_controller, Note::new(NoteName::F, 4));
        notify_bus(&update_bus);

        std::thread::sleep(std::time::Duration::from_secs(2));

        control_waveform(&source1_controller, Note::new(NoteName::C, 4));
        control_waveform(&source2_controller, Note::new(NoteName::A, 4));
        control_waveform(&source3_controller, Note::new(NoteName::D, 4));
        notify_bus(&update_bus);

        std::thread::sleep(std::time::Duration::from_secs(2));

        control_waveform(&source1_controller, Note::new(NoteName::A, 3));
        control_waveform(&source2_controller, Note::new(NoteName::G, 4));
        control_waveform(&source3_controller, Note::new(NoteName::B, 4));
        notify_bus(&update_bus);

        std::thread::sleep(std::time::Duration::from_secs(2));

        control_waveform(&source1_controller, Note::new(NoteName::G, 2));
        control_waveform(&source2_controller, Note::new(NoteName::C, 3));
        control_waveform(&source3_controller, Note::new(NoteName::E, 4));
        notify_bus(&update_bus);

        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    info!("Yage2 Engine finished");
}
