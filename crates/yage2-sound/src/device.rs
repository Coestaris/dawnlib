use crate::backend::{
    BackendDevice, BackendDeviceTrait, BackendSpecificConfig, CreateBackendConfig,
};
use crate::control::DeviceController;
use crate::dsp::bus::Bus;
use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::error::{DeviceCloseError, DeviceCreationError, DeviceOpenError};
use crate::ringbuf::RingBuffer;
use crate::sample::{InterleavedSample, InterleavedSampleBuffer, PlanarBlock, Sample};
use crate::{SampleType, BLOCK_SIZE, CHANNELS_COUNT, DEVICE_BUFFER_SIZE, RING_BUFFER_SIZE};
use log::{debug, info, warn};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::thread::Builder;
use yage2_core::profile::{MinMaxProfiler, PeriodProfiler, TickProfiler};

const WATERMARK_THRESHOLD: usize = RING_BUFFER_SIZE / 4;

pub struct DeviceConfig {
    pub backend_specific: BackendSpecificConfig,
    pub master_bus: Arc<Mutex<Bus>>,
    pub device_controller: Arc<DeviceController>,
    pub profiler_handler: Option<fn(&ProfileFrame)>,
    pub sample_rate: u32,
}

struct Profilers {
    gen_time: PeriodProfiler,
    gen_tps: TickProfiler,
    write_bulk: PeriodProfiler,
    events: PeriodProfiler,
    events_tps: TickProfiler,
    available_minmax: MinMaxProfiler,
    reader_tps: TickProfiler,
}

impl Default for Profilers {
    fn default() -> Self {
        Profilers {
            gen_time: PeriodProfiler::new(0.2),
            gen_tps: TickProfiler::new(1.0),
            write_bulk: PeriodProfiler::new(0.2),
            events: PeriodProfiler::new(0.2),
            events_tps: TickProfiler::new(1.0),
            available_minmax: MinMaxProfiler::new(),
            reader_tps: TickProfiler::new(1.0),
        }
    }
}

pub struct ProfileFrame {
    // Time consumed by the generator to produce a block of samples (in milliseconds)
    pub gen_min: f32,
    pub gen_av: f32,
    pub gen_max: f32,

    // Number of ticks per second the generator was called
    pub gen_tps_min: f32,
    pub gen_tps_av: f32,
    pub gen_tps_max: f32,

    // Time consumed by the write bulk operation (in milliseconds)
    pub write_bulk_min: f32,
    pub write_bulk_av: f32,
    pub write_bulk_max: f32,

    // Time consumed by the event processing (in milliseconds)
    pub events_min: f32,
    pub events_av: f32,
    pub events_max: f32,

    // Number of events processed per second
    pub events_tps_min: f32,
    pub events_tps_av: f32,
    pub events_tps_max: f32,

    // Minimum, average, and maximum number of available samples in the ring buffer
    pub available_min: f32,
    pub available_av: f32,
    pub available_max: f32,

    // Number of ticks per second the reader was called
    pub reader_tps_min: f32,
    pub reader_tps_av: f32,
    pub reader_tps_max: f32,

    // Device parameters
    pub sample_rate: u32,
    pub buffer_size: usize,
    pub channels: u8,
    pub block_size: usize,
}

pub struct Device {
    // The backend device that handles audio output
    backend: BackendDevice<SampleType>,

    // The ring buffer that stores audio samples
    // used for inter-thread communication between the
    // generator and the reader
    ring_buffer: Arc<RingBuffer>,

    // The master bus used for audio generation
    master_bus: Arc<Mutex<Bus>>,

    // The sample-rate of the audio device and all the audio processing
    sample_rate: u32,

    // The controller that allows user to control buses and more
    controller: Arc<DeviceController>,

    // Threads for audio generation, event processing, and statistics
    stop_signal: Arc<AtomicBool>,
    generator_thread: Option<std::thread::JoinHandle<()>>,
    events_thread: Option<std::thread::JoinHandle<()>>,
    statistics_thread: Option<std::thread::JoinHandle<()>>,

    // Profiler for collecting statistics about the audio processing
    profiler_handler: Option<fn(&ProfileFrame)>,
    profilers: Arc<Profilers>,
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Device {
    pub fn new(config: DeviceConfig) -> Result<Self, DeviceCreationError> {
        if config.sample_rate == 0 {
            return Err(DeviceCreationError::InvalidSampleRate(config.sample_rate));
        }

        let backend_config = CreateBackendConfig {
            backend_specific: config.backend_specific,
            sample_rate: config.sample_rate,
            channels: CHANNELS_COUNT,
            buffer_size: DEVICE_BUFFER_SIZE,
        };

        let backend_device = BackendDevice::<SampleType>::new(backend_config)
            .map_err(DeviceCreationError::BackendSpecific)?;

        Ok(Device {
            backend: backend_device,
            ring_buffer: Arc::new(RingBuffer::new()),
            master_bus: Arc::clone(&config.master_bus),

            controller: config.device_controller,
            sample_rate: config.sample_rate,

            stop_signal: Arc::new(AtomicBool::new(false)),
            generator_thread: None,
            events_thread: None,
            statistics_thread: None,

            profiler_handler: config.profiler_handler,
            profilers: Arc::new(Profilers::default()),
        })
    }

    fn spawn_generator_thread(&mut self) -> Result<(), DeviceOpenError> {
        let buffer = Arc::clone(&self.ring_buffer);
        let master = Arc::clone(&self.master_bus);
        let sample_rate = self.sample_rate;
        let profiler = Arc::clone(&self.profilers);
        let tick = move |sample_index: usize| {
            profiler.gen_tps.tick(1);

            // Generate a block of planar samples
            // All the processing is done in f32 format
            let mut planar_block = PlanarBlock::<f32> {
                samples: [[0.0; BLOCK_SIZE]; CHANNELS_COUNT as usize],
            };
            let block_info = BlockInfo {
                sample_index,
                sample_rate,
            };

            // Generate samples using the master bus
            let bus_guard = master.lock().unwrap();
            profiler.gen_time.start();
            bus_guard.generate(&mut planar_block, &block_info);
            profiler.gen_time.end();
            drop(bus_guard);

            // Convert planar samples to interleaved format
            // Convert to the SampleType if necessary
            let mut interleaved_samples: [InterleavedSample<SampleType>; BLOCK_SIZE] =
                [InterleavedSample::default(); BLOCK_SIZE];
            for i in 0..BLOCK_SIZE {
                for channel in 0..CHANNELS_COUNT as usize {
                    let sample = planar_block.samples[channel][i];
                    interleaved_samples[i].channels[channel] = SampleType::from_f32(sample);
                }
            }

            // Write interleaved samples to the ring buffer
            profiler.write_bulk.start();
            let guard = buffer.write_bulk_wait(BLOCK_SIZE);
            guard.write_samples(&interleaved_samples);
            profiler.write_bulk.end();
        };

        let stop_signal = Arc::clone(&self.stop_signal);
        self.generator_thread = Some(
            Builder::new()
                .name("aud_gen".into())
                .spawn(move || {
                    let mut sample_index = 0;
                    loop {
                        // Check if the stop signal is set
                        if stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio generator thread stopping");
                            break;
                        }

                        // Generate a block of samples
                        tick(sample_index);

                        // Increment the sample index for the next block
                        sample_index += BLOCK_SIZE;
                    }
                })
                .map_err(|_| DeviceOpenError::GeneratorThreadSpawnError)?,
        );

        Ok(())
    }

    fn spawn_events_thread(&mut self) -> Result<(), DeviceOpenError> {
        let controller = Arc::clone(&self.controller);
        let master = Arc::clone(&self.master_bus);
        let profiler = Arc::clone(&self.profilers);
        let tick = move || {
            profiler.events_tps.tick(1);

            // Wait for the update bus to signal that there are new events
            controller.wait_for_update();

            // Process events in the main bus
            debug!("Processing events in the main bus");
            let mut bus_guard = master.lock().unwrap();
            profiler.events.start();
            bus_guard.dispatch_events();
            profiler.events.end();
            drop(bus_guard);
        };

        let events_stop_signal = Arc::clone(&self.stop_signal);
        self.events_thread = Some(
            Builder::new()
                .name("aud_events".into())
                .spawn(move || {
                    loop {
                        // Check if the stop signal is set
                        if events_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio events thread stopping");
                            break;
                        }

                        // Process events
                        tick();
                    }
                })
                .map_err(|_| DeviceOpenError::EventThreadSpawnError)?,
        );
        Ok(())
    }

    fn spawn_statistics_thread(&mut self) -> Result<(), DeviceOpenError> {
        let profiler_handler = match &self.profiler_handler {
            Some(handler) => handler.clone(),
            None => {
                warn!("No profiler handler provided, statistics thread will not log data");
                return Ok(());
            }
        };

        let statistics_profiler = Arc::clone(&self.profilers);
        let statistics_stop_signal = Arc::clone(&self.stop_signal);
        let sample_rate = self.sample_rate;
        let tick = move || {
            statistics_profiler.gen_tps.update();
            statistics_profiler.events_tps.update();

            let (proc_min, proc_av, proc_max) = statistics_profiler.gen_time.get_stat();
            let (proc_tps_min, proc_tps_av, proc_tps_max) = statistics_profiler.gen_tps.get_stat();
            let (write_bulk_min, write_bulk_av, write_bulk_max) =
                statistics_profiler.write_bulk.get_stat();
            let (events_min, events_av, events_max) = statistics_profiler.events.get_stat();
            let (events_tps_min, events_tps_av, events_tps_max) =
                statistics_profiler.events_tps.get_stat();
            let (available_min, available_av, available_max) =
                statistics_profiler.available_minmax.get_stat();
            let (reader_tps_min, reader_tps_av, reader_tps_max) =
                statistics_profiler.reader_tps.get_stat();

            let frame = ProfileFrame {
                gen_min: proc_min,
                gen_av: proc_av,
                gen_max: proc_max,
                gen_tps_min: proc_tps_min,
                gen_tps_av: proc_tps_av,
                gen_tps_max: proc_tps_max,
                write_bulk_min,
                write_bulk_av,
                write_bulk_max,
                events_min,
                events_av,
                events_max,
                events_tps_min,
                events_tps_av,
                events_tps_max,
                available_min,
                available_av,
                available_max,
                reader_tps_min,
                reader_tps_av,
                reader_tps_max,
                sample_rate,
                buffer_size: DEVICE_BUFFER_SIZE,
                channels: CHANNELS_COUNT,
                block_size: BLOCK_SIZE,
            };

            profiler_handler(&frame);

            // Sleep for a short duration to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(1000));
        };

        self.statistics_thread = Some(
            Builder::new()
                .name("aud_stats".into())
                .spawn(move || {
                    loop {
                        // Check if the stop signal is set
                        if statistics_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio statistics thread stopping");
                            break;
                        }

                        // Collect and log statistics
                        tick();
                    }
                })
                .map_err(|_| DeviceOpenError::StatisticsThreadSpawnError)?,
        );

        Ok(())
    }

    fn spawn_reader(&mut self) -> Result<(), DeviceOpenError> {
        let reader_profiler = Arc::clone(&self.profilers);
        let reader_buffer = Arc::clone(&self.ring_buffer);
        let func = move |buffer: &mut InterleavedSampleBuffer<SampleType>| {
            reader_profiler.reader_tps.tick(1);

            let guard = reader_buffer.read_bulk();
            let (read, available) = guard.read_or_silence(buffer.samples);
            if read < buffer.samples.len() {
                warn!(
                    "Buffer underflow: requested {} samples, got only {}",
                    buffer.samples.len(),
                    read
                );
            }

            reader_profiler.available_minmax.update(available as u64);
            if available < WATERMARK_THRESHOLD {
                warn!("Low available samples in ring buffer: {}", available);
            }
        };

        self.backend
            .open(func)
            .map_err(DeviceOpenError::BackendSpecific)?;

        Ok(())
    }

    pub fn open(&mut self) -> Result<(), DeviceOpenError> {
        info!("Opening audio device");

        self.spawn_events_thread()?;
        self.spawn_generator_thread()?;
        self.spawn_statistics_thread()?;
        self.spawn_reader()?;

        Ok(())
    }

    pub fn close(&mut self) -> Result<(), DeviceCloseError> {
        info!("Closing audio device");
        self.backend
            .close()
            .map_err(DeviceCloseError::BackendSpecific)?;

        // Notify the generator thread to stop
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Relaxed);
        // Unlock the ring buffer to allow the generator thread to finish
        self.ring_buffer.poison();
        // Notify the events thread to stop
        self.controller.reset();

        // Wait for the generator thread to finish
        if let Some(thread) = self.generator_thread.take() {
            if let Err(e) = thread.join() {
                warn!("Failed to join audio generator thread: {:?}", e);
            }
        }
        if let Some(thread) = self.events_thread.take() {
            if let Err(e) = thread.join() {
                warn!("Failed to join audio events thread: {:?}", e);
            }
        }
        if let Some(thread) = self.statistics_thread.take() {
            if let Err(e) = thread.join() {
                warn!("Failed to join audio statistics thread: {:?}", e);
            }
        }

        Ok(())
    }
}
