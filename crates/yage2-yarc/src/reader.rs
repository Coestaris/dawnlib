use crate::structures::ResourceMetadata;
use log::info;
use std::collections::HashMap;
use std::io::Read;
use tar::Archive;

#[derive(Default)]
pub struct Container {
    pub name: String,
    pub binary: Vec<u8>,
    pub metadata: ResourceMetadata,
}

#[derive(Debug)]
pub enum ReadError {
    IoError(std::io::Error),
    ReadTarError(std::io::Error),
    DecodeError(String),
    TomlError(toml::de::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::IoError(e) => write!(f, "I/O error: {}", e),
            ReadError::ReadTarError(e) => write!(f, "Failed to read tar entry: {}", e),
            ReadError::DecodeError(e) => write!(f, "Failed to decode contents: {}", e),
            ReadError::TomlError(e) => write!(f, "Failed to parse TOML: {}", e),
        }
    }
}

fn read_from_reader<R>(reader: R) -> Result<HashMap<String, Container>, ReadError>
where
    R: Read,
{
    let mut archive = Archive::new(reader);

    let mut containers: HashMap<String, Container> = HashMap::new();
    for entry in archive.entries().map_err(ReadError::ReadTarError)? {
        let mut entry = entry.map_err(ReadError::ReadTarError)?;

        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(ReadError::ReadTarError)?;

        let path = entry.path().map_err(ReadError::IoError)?;

        // Manifest is not actually needed for reading resources,
        // but we still read it to ensure compatibility with the format.
        if path != std::path::Path::new(".manifest.toml") {
            // Metadata or binary
            let we = path
                .file_stem()
                .and_then(|f| f.to_str())
                .unwrap()
                .to_string();
            let extension = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let container = containers
                .entry(we.clone())
                .or_insert_with(|| Container::default());

            if extension == "toml" {
                // Metadata file
                let string = String::from_utf8(contents)
                    .map_err(|e| ReadError::DecodeError(e.to_string()))?;
                container.metadata = toml::from_str(&string).map_err(ReadError::TomlError)?;
            } else {
                // Binary file
                container.binary = contents;
            }
        }
    }

    Ok(containers)
}

pub fn read_from_file_compressed<P: AsRef<std::path::Path>>(
    input: P,
) -> Result<HashMap<String, Container>, ReadError> {
    let file = std::fs::File::open(input).unwrap();
    let buf_reader = std::io::BufReader::new(file);
    let decoder = flate2::read::GzDecoder::new(buf_reader);

    read_from_reader(decoder)
}

pub fn read_from_file_uncompressed<P: AsRef<std::path::Path>>(
    input: P,
) -> Result<HashMap<String, Container>, ReadError> {
    let file = std::fs::File::open(input).unwrap();
    let buf_reader = std::io::BufReader::new(file);

    read_from_reader(buf_reader)
}

pub fn read<P: AsRef<std::path::Path>>(input: P) -> Result<HashMap<String, Container>, ReadError> {
    match read_from_file_compressed(input.as_ref()) {
        Err(ReadError::ReadTarError(e)) => {
            // Try to read as non-compressed tar
            info!(
                "Failed to read as compressed tar, trying uncompressed: {}",
                e
            );
            read_from_file_uncompressed(input)
        }
        any => any,
    }
}
