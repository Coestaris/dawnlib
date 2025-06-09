extern crate core;

use crate::win32::Win32Window;
use crate::window::Window;
use ansi_term::Colour::{Blue, Cyan, Green, Red, Yellow};
use log::{Level, LevelFilter, Metadata, Record, info};

mod vulkan;
mod win32;
mod window;

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

            println!(
                "[{}][{}][{}]: {} [{}:{}]",
                Cyan.paint(
                    chrono::Local::now()
                        .format("%Y-%m-%d %H:%M:%S%.3f")
                        .to_string()
                ),
                Yellow.paint(std::thread::current().name().unwrap_or("main")),
                colored_level(record.level()).paint(record.level().to_string()),
                record.args(),
                Green.paint(record.file().unwrap_or("unknown")),
                Green.paint(record.line().unwrap_or(0).to_string())
            );
        }
    }

    fn flush(&self) {}
}

fn main() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap();

    info!("Yage2 Engine started");

    let window = Win32Window::new("Yage2 Engine", 1280, 720).unwrap();
    window.event_loop().unwrap();
}
