use std::sync::Arc;
use log::info;
use wav::BitDepth;
use yage2_core::resources::{Resource, ResourceMetadata};
use crate::sample::Sample;

fn convert<F>(input: Vec<F>) -> Vec<f32>
where
    F: Sample + Copy,
{
    let mut output = Vec::with_capacity(input.len());
    for sample in input {
        output.push(sample.to_f32());
    }

    output
}

pub fn parse_resource(metadata: &ResourceMetadata, raw: &[u8]) -> Result<Resource, String> {
    match metadata.resource_type {
        yage2_core::resources::ResourceType::Audio => {
            let mut buf_reader = std::io::Cursor::new(raw);
            let (header, data) = wav::read(&mut buf_reader)
                .map_err(|e| format!("Failed to read WAV file: {}", e))?;

            let samples = match data {
                // BitDepth::Eight(val_u8) => { convert::<u8>(val_u8) },
                BitDepth::Sixteen(val_i16) => { convert::<i16>(val_i16) },
                BitDepth::TwentyFour(val_i32) => { convert::<i32>(val_i32) },
                BitDepth::ThirtyTwoFloat(val_f32) => { convert::<f32>(val_f32) }
                _ => {
                    return Err("No audio data found in WAV file".to_string());
                }
            };

            info!("Parsed WAV file: {} Hz, {} channels, {} samples",
                header.sampling_rate, header.channel_count, samples.len()
            );
            
            Ok(Resource::new(ClipResource {
                sample_rate: header.sampling_rate,
                len: samples.len() as u32,
                channels: header.channel_count,
                data: samples,
            }))
        }
        _ => Err(format!(
            "Unsupported resource type: {:?}",
            metadata.resource_type
        )),
    }
}

pub fn finalize_resource(metadata: &ResourceMetadata, res: &Resource) -> Result<(), String> {
    Ok(())
}

pub struct ClipResource {
    pub sample_rate: u32,
    pub len: u32, // Length in samples
    pub channels: u16,
    pub data: Vec<f32>,
}
