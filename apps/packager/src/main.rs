use clap::{command, Parser};
use common::logging::CommonLogger;
use std::path::PathBuf;
use yage2_yarc::{ChecksumAlgorithm, Compression, ReadMode, WriteOptions};

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

    /// Embed author of the YARC container content
    #[arg(long)]
    content_author: Option<String>,
    /// Embed description of the YARC container content
    #[arg(long)]
    content_description: Option<String>,
    /// Embed version of the YARC container content
    #[arg(long)]
    content_version: Option<String>,
    /// Embed license of the YARC container content
    #[arg(long)]
    content_license: Option<String>,
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
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
        PathBuf::from(cli.input),
        WriteOptions {
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
            author: cli.content_author,
            description: cli.content_description,
            version: cli.content_version,
            license: cli.content_license,
        },
        PathBuf::from(cli.output),
    )
    .unwrap_or_else(|err| {
        log::error!("Failed to create YARC: {}", err);
        std::process::exit(1);
    });
}
