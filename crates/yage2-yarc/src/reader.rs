use crate::structures::{Manifest, ResourceMetadata};
use log::info;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::option::Option;
use yage2_core::resources::Resource;

#[derive(Default)]
pub struct Container {
    pub name: String,
    pub binary: Vec<u8>,
    pub metadata: ResourceMetadata,
}

fn list_tar_entries<P: AsRef<std::path::Path>>(
    input_dir: P,
) -> Result<(Manifest, HashMap<String, Container>), String> {
    let file = std::fs::File::open(input_dir).unwrap();
    let buf_reader = std::io::BufReader::new(file);
    let decoder = flate2::read::GzDecoder::new(buf_reader);
    let mut archive = tar::Archive::new(decoder);

    let mut manifest: Option<Manifest> = None;
    let mut containers: HashMap<String, Container> = HashMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();

        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).unwrap();

        let path = entry.path().unwrap();

        if path == std::path::Path::new(".manifest.toml") {
            let string = String::from_utf8(contents).unwrap();
            manifest = toml::from_str(&string).unwrap();
        } else {
            // Metadata or binary
            let we = path
                .file_stem()
                .and_then(|f| f.to_str())
                .ok_or_else(|| "Invalid file name".to_string())?
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
                let string = String::from_utf8(contents).unwrap();
                container.metadata = toml::from_str(&string).unwrap_or_default();
            } else {
                // Binary file
                container.binary = contents;
            }
        }
    }

    Ok((manifest.unwrap(), containers))
}

pub fn read<P: AsRef<std::path::Path>>(input: P) -> Result<HashMap<String, Container>, String> {
    let (manifest, containers) = list_tar_entries(input).unwrap();
    Ok(containers)
}
