use crate::sample::{InterleavedSample, Sample};
use crate::CHANNELS_COUNT;

fn to_interleaved_f32<F>(
    interleaved_raw: Vec<F>,
    raw_channels: usize,
) -> Vec<InterleavedSample<f32>>
where
    F: Sample + Copy,
{
    let mut output = Vec::with_capacity(interleaved_raw.len());
    let mut i = 0;
    while i < interleaved_raw.len() {
        let mut sample = InterleavedSample::<f32>::default();

        // Our interleaved sample has a fixed number of channels that
        // can be less or greater than the input channels.
        // In case of fewer channels, we will just copy last sample to the rest of the channels.
        // In case of more channels, we will just ignore the extra channels.
        for j in 0..CHANNELS_COUNT as usize {
            if j < raw_channels {
                sample.channels[j] = F::to_f32(interleaved_raw[i + j]);
            } else {
                // If we have fewer channels, copy the last sample to the rest of the channels
                sample.channels[j] = sample.channels[raw_channels - 1];
            }
        }

        // Stride the input by the number of channels
        i += raw_channels;

        output.push(sample);
    }

    output
}

pub struct ClipResource {
    pub sample_rate: u32,
    pub len: usize, // Length in interleaved samples
    pub channels: u16,
    pub data: Vec<InterleavedSample<f32>>,
}

#[cfg(feature = "resources-wav")]
pub(crate) mod wav {
    use crate::resources::{to_interleaved_f32, ClipResource};
    use crate::CHANNELS_COUNT;
    use log::{error, warn};
    use yage2_core::resources::{Resource, ResourceFactory, ResourceHeader};

    pub(crate) struct WAVResourceFactory {
        sample_rate: u32,
    }

    impl WAVResourceFactory {
        pub fn new(sample_rate: u32) -> Self {
            WAVResourceFactory { sample_rate }
        }
    }

    impl ResourceFactory for WAVResourceFactory {
        fn parse(&self, header: &ResourceHeader, raw: &[u8]) -> Result<Resource, String> {
            let mut buf_reader = std::io::Cursor::new(raw);
            match hound::WavReader::new(&mut buf_reader) {
                Ok(mut reader) => {
                    let spec = reader.spec();
                    if spec.sample_rate != self.sample_rate {
                        return Err(format!(
                            "WAV {} sample rate mismatch: expected {}, got {}. Resampling is currently not supported.",
                            header.name, self.sample_rate, spec.sample_rate
                        ));
                    }

                    let data = match (spec.sample_format, spec.bits_per_sample) {
                        (hound::SampleFormat::Float, 32) => {
                            let samples: Vec<f32> =
                                reader.samples::<f32>().map(|s| s.unwrap()).collect();

                            to_interleaved_f32(samples, spec.channels as usize)
                        }
                        (hound::SampleFormat::Int, 16) => {
                            let samples: Vec<i16> =
                                reader.samples::<i16>().map(|s| s.unwrap()).collect();

                            to_interleaved_f32(samples, spec.channels as usize)
                        }
                        (hound::SampleFormat::Int, 24) => {
                            let samples: Vec<i24::i24> = reader
                                .samples::<i32>()
                                .map(|s| i24::i24::try_from_i32(s.unwrap()).unwrap())
                                .collect();

                            to_interleaved_f32(samples, spec.channels as usize)
                        }
                        (hound::SampleFormat::Int, 32) => {
                            let samples: Vec<i32> =
                                reader.samples::<i32>().map(|s| s.unwrap()).collect();

                            to_interleaved_f32(samples, spec.channels as usize)
                        }
                        _ => {
                            return Err(format!(
                                "Unsupported WAV {} format: {:?} with {} bits per sample",
                                header.name, spec.sample_format, spec.bits_per_sample
                            ));
                        }
                    };

                    Ok(Resource::new(ClipResource {
                        sample_rate: self.sample_rate,
                        len: data.len(),
                        channels: CHANNELS_COUNT as u16,
                        data,
                    }))
                }

                Err(e) => Err(format!("Error parsing WAV {} file: {}", header.name, e)),
            }
        }

        fn finalize(&self, header: &ResourceHeader, resource: &Resource) -> Result<(), String> {
            Ok(())
        }
    }
}

#[cfg(feature = "resources-flac")]
pub(crate) mod flac {
    use crate::resources::{to_interleaved_f32, ClipResource};
    use crate::CHANNELS_COUNT;
    use log::{error, warn};
    use yage2_core::resources::{Resource, ResourceFactory, ResourceHeader};

    pub(crate) struct FLACResourceFactory {
        sample_rate: u32,
    }

    impl FLACResourceFactory {
        pub fn new(sample_rate: u32) -> Self {
            FLACResourceFactory { sample_rate }
        }
    }

    impl ResourceFactory for FLACResourceFactory {
        fn parse(&self, header: &ResourceHeader, raw: &[u8]) -> Result<Resource, String> {
            // Placeholder for FLAC parsing logic
            // In a real implementation, this would parse the FLAC file and convert it to interleaved samples.
            return Err(format!(
                "FLAC parsing not implemented for resource: {}",
                header.name
            ));
        }

        fn finalize(&self, header: &ResourceHeader, resource: &Resource) -> Result<(), String> {
            Ok(())
        }
    }
}

#[cfg(feature = "resources-ogg")]
pub(crate) mod ogg {
    use crate::resources::{to_interleaved_f32, ClipResource};
    use crate::CHANNELS_COUNT;
    use log::{error, warn};
    use yage2_core::resources::{Resource, ResourceFactory, ResourceHeader};

    pub(crate) struct OGGResourceFactory {
        sample_rate: u32,
    }

    impl OGGResourceFactory {
        pub fn new(sample_rate: u32) -> Self {
            OGGResourceFactory { sample_rate }
        }
    }

    impl ResourceFactory for OGGResourceFactory {
        fn parse(&self, header: &ResourceHeader, raw: &[u8]) -> Result<Resource, String> {
            // Placeholder for OGG parsing logic
            // In a real implementation, this would parse the OGG file and convert it to interleaved samples.
            return Err(format!(
                "OGG parsing not implemented for resource: {}",
                header.name
            ));
        }

        fn finalize(&self, header: &ResourceHeader, resource: &Resource) -> Result<(), String> {
            Ok(())
        }
    }
}
