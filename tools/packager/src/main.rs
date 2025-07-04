use ansi_term::Color::{Blue, Cyan, Green, Red, Yellow};
use clap::{command, Parser};
use log::{Level, Metadata, Record};
use yage2_core::utils::format_now;
use yage2_yarc::structures::{ChecksumAlgorithm, Compression, ReadMode, YARCWriteOptions};

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

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CLI {
    /// Input directory to create the YARC from
    #[arg(short, long)]
    input: String,

    /// Output file for the YARC
    /// This will be the name of the YARC file created
    #[arg(short, long)]
    output: String,

    /// Use compression for the YARC (true by default)
    #[arg(short, long)]
    compression: Option<bool>,

    /// Use recursive read mode (true by default)
    #[arg(short, long)]
    recursive: Option<bool>,

    /// Checksum algorithm to use for the YARC
    #[arg(short = 'a', long)]
    checksum_algorithm: Option<String>,
}

fn main() {
    // Initialize the logger
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    // Example usage of the logger
    log::info!("Starting the YARC packager...");
    let cli = CLI::parse();
    log::info!("Input directory: {}", cli.input);
    log::info!("Output file: {}", cli.output);

    let checksum_str = cli.checksum_algorithm.unwrap_or("md5".to_string());
    let checksum = match checksum_str.as_str() {
        "md5" => ChecksumAlgorithm::Md5,
        "blake3" => ChecksumAlgorithm::Blake3,
        _ => {
            log::error!("Unsupported checksum algorithm: {}", checksum_str);
            return;
        }
    };

    yage2_yarc::write_from_directory(
        cli.input,
        YARCWriteOptions {
            compression: if cli.compression.unwrap_or(true) {
                Compression::Gzip
            } else {
                Compression::None
            },
            read_mode: if cli.recursive.unwrap_or(true) {
                ReadMode::Recursive
            } else {
                ReadMode::Flat
            },
            checksum_algorithm: checksum,
        },
        cli.output,
    )
    .unwrap_or_else(|err| {
        log::error!("Failed to create YARC: {}", err);
        std::process::exit(1);
    });
}
