extern crate core;

use ansi_term::Colour::{Blue, Cyan, Green, Red, Yellow};
use log::{info, Level, LevelFilter, Metadata, Record};
use yage2::core::utils::format_now;
use yage2::engine::application::Application;

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
                Yellow.paint(std::thread::current().name().unwrap_or("main")).to_string(),
                colored_level(record.level()).paint(record.level().to_string()).to_string(),
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

    let application_config = yage2::engine::application::ApplicationConfig {
        window_config: yage2::engine::window::WindowConfig {
            title: "Yage2 Engine".to_string(),
            width: 1280,
            height: 720,
        },
    };
    let app = yage2::create_app!(application_config).unwrap();
    app.run().unwrap();

    info!("Yage2 Engine finished");
}
