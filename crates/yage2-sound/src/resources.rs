use crate::sample::{InterleavedSample, Sample};
use crate::CHANNELS_COUNT;

#[cfg(feature = "resources-resample")]
fn resample(
    data: Vec<InterleavedSample<f32>>,
    src_sample_rate: u32,
    dest_sample_rate: u32,
) -> Result<Vec<InterleavedSample<f32>>, String> {
    use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

    if src_sample_rate == dest_sample_rate {
        return Ok(data);
    }

    /// Converts interleaved audio to planar format
    fn deinterleave(input: &[InterleavedSample<f32>]) -> Vec<Vec<f32>> {
        let mut planar = vec![vec![0.0; input.len()]; CHANNELS_COUNT as usize];
        for (i, frame) in input.iter().enumerate() {
            for c in 0..CHANNELS_COUNT {
                planar[c as usize][i] = frame.channels[c as usize];
            }
        }
        planar
    }

    /// Converts planar format back to interleaved
    fn interleave(planar: &[Vec<f32>]) -> Vec<InterleavedSample<f32>> {
        let len = planar[0].len();
        let mut output = vec![InterleavedSample::<f32>::default(); len];
        for c in 0..CHANNELS_COUNT {
            for i in 0..len {
                output[i].channels[c as usize] = planar[c as usize][i];
            }
        }
        output
    }

    let planar_data = deinterleave(&data);

    let params = SincInterpolationParameters {
        sinc_len: 64,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: rubato::WindowFunction::Hann,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        dest_sample_rate as f64 / src_sample_rate as f64,
        2.0, // tolerance
        params,
        data.len(),
        CHANNELS_COUNT as usize,
    )
    .map_err(|e| format!("Failed to create resampler: {}", e))?;

    let resampled_planar = resampler
        .process(&planar_data, None)
        .map_err(|e| format!("Failed to resample audio: {}", e))?;

    Ok(interleave(&resampled_planar))
}

#[cfg(not(feature = "resources-resample"))]
fn resample(
    _: Vec<InterleavedSample<f32>>,
    _: u32,
    _: u32,
) -> Result<Vec<InterleavedSample<f32>>, String> {
    Err("Resampling is not enabled. Please enable the 'resample-resources' feature or use a WAV file with the same sample rate as the audio device.".to_string())
}

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
pub(crate) mod wav
{
    use log::warn;
    use yage2_core::resources::{Resource, ResourceFactory, ResourceMetadata};
    use crate::CHANNELS_COUNT;
    use crate::resources::{resample, to_interleaved_f32, ClipResource};

    pub(crate) struct WAVResourceFactory {
        sample_rate: u32,
    }

    impl WAVResourceFactory {
        pub fn new(sample_rate: u32) -> Self {
            WAVResourceFactory { sample_rate }
        }
    }

    impl ResourceFactory for WAVResourceFactory {
        fn parse(&self, metadata: &ResourceMetadata, raw: &[u8]) -> Result<Resource, String> {
            let mut buf_reader = std::io::Cursor::new(raw);
            match hound::WavReader::new(&mut buf_reader) {
                Ok(mut reader) => {
                    let spec = reader.spec();
                    if spec.sample_rate != self.sample_rate {
                        warn!(
                            "Resampling from {} Hz to {} Hz of the WAV file '{}'",
                            spec.sample_rate, self.sample_rate, metadata.name
                        );
                    }

                    let data = match (spec.sample_format, spec.bits_per_sample) {
                        (hound::SampleFormat::Float, 32) => {
                            let samples: Vec<f32> =
                                reader.samples::<f32>().map(|s| s.unwrap()).collect();
                            resample(
                                to_interleaved_f32(samples, spec.channels as usize),
                                spec.sample_rate,
                                self.sample_rate,
                            )
                        }
                        (hound::SampleFormat::Int, 16) => {
                            let samples: Vec<i16> =
                                reader.samples::<i16>().map(|s| s.unwrap()).collect();
                            resample(
                                to_interleaved_f32(samples, spec.channels as usize),
                                spec.sample_rate,
                                self.sample_rate,
                            )
                        }
                        (hound::SampleFormat::Int, 24) => {
                            let samples: Vec<i24::i24> = reader
                                .samples::<i32>()
                                .map(|s| i24::i24::try_from_i32(s.unwrap()).unwrap())
                                .collect();
                            resample(
                                to_interleaved_f32(samples, spec.channels as usize),
                                spec.sample_rate,
                                self.sample_rate,
                            )
                        }
                        (hound::SampleFormat::Int, 32) => {
                            let samples: Vec<i32> =
                                reader.samples::<i32>().map(|s| s.unwrap()).collect();
                            resample(
                                to_interleaved_f32(samples, spec.channels as usize),
                                spec.sample_rate,
                                self.sample_rate,
                            )
                        }
                        _ => {
                            return Err(format!(
                                "Unsupported WAV {} format: {:?} with {} bits per sample",
                                metadata.name, spec.sample_format, spec.bits_per_sample
                            ));
                        }
                    }
                        .map_err(|e| format!("Error resampling WAV {} file: {}", metadata.name, e))?;

                    Ok(Resource::new(ClipResource {
                        sample_rate: self.sample_rate,
                        len: data.len(),
                        channels: CHANNELS_COUNT as u16,
                        data,
                    }))
                }

                Err(e) => Err(format!("Error parsing WAV {} file: {}", metadata.name, e)),
            }
        }

        fn finalize(&self, metadata: &ResourceMetadata, resource: &Resource) -> Result<(), String> {
            Ok(())
        }
    }
}
    