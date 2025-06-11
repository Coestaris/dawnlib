extern crate core;

mod graphics;
mod platforms;
mod utils;

use crate::graphics::window::Window;
use ansi_term::Colour::{Blue, Cyan, Green, Red, Yellow};
use log::{info, Level, LevelFilter, Metadata, Record};
use std::time::SystemTime;
use crate::utils::format_now;

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

            const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
            let formatted_date = format_now(DATE_FORMAT).unwrap_or("unknown".to_string());

            println!(
                "[{}][{}][{}]: {} [{}:{}]",
                Cyan.paint(formatted_date),
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

    let window = create_window!("Yage2 Engine", 1280, 720).unwrap();
    window.event_loop().unwrap();

    info!("Yage2 Engine finished");
}
