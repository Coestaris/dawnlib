use crate::dsp::bus::Bus;
use crate::dsp::{Control, Generator};
use crate::{
    dsp::BlockInfo,
    error::{DeviceCloseError, DeviceCreationError, DeviceOpenError},
    ringbuf::RingBuffer,
    sample::{InterleavedSample, PlanarBlock, Sample},
    BackendDevice, BackendSpecificConfig, BackendSpecificError, SampleType, BLOCK_SIZE,
    CHANNELS_COUNT, DEVICE_BUFFER_SIZE, RING_BUFFER_SIZE,
};
use log::{debug, info, warn};
use std::sync::{atomic::AtomicBool, Arc, Condvar, Mutex};
use std::thread::Builder;
use yage2_core::profile::{MinMaxProfiler, PeriodProfiler, TickProfiler};

const WATERMARK_THRESHOLD: usize = RING_BUFFER_SIZE / 4;

#[allow(dead_code)]
pub(crate) struct CreateBackendConfig {
    pub backend_specific: BackendSpecificConfig,
    pub sample_rate: u32,
    pub channels: u8,
    pub buffer_size: usize,
}

/// This struct represents a buffer of interleaved samples.
/// It is used to store audio samples in a format where
/// each sample contains data for all channels interleaved together.
/// For example: r.0, l.0, r.1, l.1, r.2, l.2, ...
/// Used for endpoint audio processing - passing data to the audio device.
/// The Amount of samples in the buffer is equal to `DEVICE_BUFFER_SIZE`.
#[repr(C)]
#[derive(Debug, Default)]
pub(crate) struct InterleavedSampleBuffer<'a, S>
where
    S: Sample,
{
    pub(crate) samples: &'a mut [InterleavedSample<S>],
    pub(crate) len: usize,
}

impl<'a, S> InterleavedSampleBuffer<'a, S>
where
    S: Sample,
{
    pub fn new(raw: &'a mut [S]) -> Option<Self> {
        // Check that the length of the raw slice is a multiple of CHANNELS_COUNT
        if raw.len() % CHANNELS_COUNT as usize != 0 {
            return None; // Invalid length for interleaved samples
        }

        let ptr = raw.as_mut_ptr() as *mut InterleavedSample<S>;
        let len = raw.len() / CHANNELS_COUNT as usize;

        let casted = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        Some(Self {
            samples: casted,
            len,
        })
    }
}

pub(crate) trait BackendDeviceTrait<S>
where
    S: Sample,
{
    fn new(cfg: CreateBackendConfig) -> Result<Self, BackendSpecificError>
    where
        Self: Sized;

    fn open<F>(&mut self, raw_fn: F) -> Result<(), BackendSpecificError>
    where
        F: FnMut(&mut InterleavedSampleBuffer<S>) + Send + 'static;

    fn close(&mut self) -> Result<(), BackendSpecificError>;
}

pub struct DeviceConfig {
    pub backend_specific: BackendSpecificConfig,
    pub main_bus: Arc<Mutex<Bus>>,
    pub update_bus: Arc<(Mutex<u8>, Condvar)>,
    pub sample_rate: u32,
}

#[derive(Default)]
struct Profiler {
    process: PeriodProfiler,
    process_tps: TickProfiler,
    write_bulk: PeriodProfiler,

    events: PeriodProfiler,
    events_tps: TickProfiler,

    available_minmax: MinMaxProfiler,
    reader_tps: TickProfiler,
}

pub struct Device {
    backend: BackendDevice<SampleType>,
    ring_buffer: Arc<RingBuffer>,
    stop_signal: Arc<AtomicBool>,
    main_bus: Arc<Mutex<Bus>>,
    sample_rate: u32,

    update_bus: Arc<(Mutex<u8>, Condvar)>,
    profiler: Arc<Profiler>,

    generator_thread: Option<std::thread::JoinHandle<()>>,
    events_thread: Option<std::thread::JoinHandle<()>>,
    statistics_thread: Option<std::thread::JoinHandle<()>>,
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
            sample_rate: config.sample_rate,
            generator_thread: None,
            events_thread: None,
            statistics_thread: None,
            update_bus: config.update_bus,
            stop_signal: Arc::new(AtomicBool::new(false)),
            main_bus: Arc::clone(&config.main_bus),
            profiler: Arc::new(Profiler {
                process: PeriodProfiler::new(0.2),
                process_tps: TickProfiler::new(1.0),
                write_bulk: PeriodProfiler::new(0.2),
                events: PeriodProfiler::new(0.2),
                events_tps: TickProfiler::new(1.0),
                available_minmax: MinMaxProfiler::new(),
                reader_tps: TickProfiler::new(1.0),
            }),
        })
    }

    pub fn open(&mut self) -> Result<(), DeviceOpenError> {
        info!("Opening audio device");

        let writer_buffer = Arc::clone(&self.ring_buffer);
        let generator_stop_signal = Arc::clone(&self.stop_signal);
        let generator_bus = Arc::clone(&self.main_bus);
        let sample_rate = self.sample_rate;
        let generator_profiler = Arc::clone(&self.profiler);
        self.generator_thread = Some(
            Builder::new()
                .name("aud_gen".into())
                .spawn(move || {
                    let mut sample_index = 0;

                    loop {
                        generator_profiler.process_tps.tick(1);

                        // Check if the stop signal is set
                        if generator_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio generator thread stopping");
                            break;
                        }

                        // Generate a block of planar samples
                        // All the processing is done in f32 format
                        let mut planar_block = PlanarBlock::<f32> {
                            samples: [[0.0; BLOCK_SIZE]; CHANNELS_COUNT as usize],
                        };
                        let block_info = BlockInfo {
                            sample_index,
                            sample_rate,
                        };

                        let bus_guard = generator_bus.lock().unwrap();
                        generator_profiler.process.start();
                        bus_guard.generate(&mut planar_block, &block_info);
                        generator_profiler.process.end();
                        drop(bus_guard);

                        sample_index += BLOCK_SIZE;

                        // Convert planar samples to interleaved format
                        // Convert to the SampleType if necessary
                        let mut interleaved_samples: [InterleavedSample<SampleType>; BLOCK_SIZE] =
                            [InterleavedSample::default(); BLOCK_SIZE];
                        for i in 0..BLOCK_SIZE {
                            for channel in 0..CHANNELS_COUNT as usize {
                                let sample = planar_block.samples[channel][i];
                                interleaved_samples[i].channels[channel] =
                                    SampleType::from_f32(sample);
                            }
                        }

                        generator_profiler.write_bulk.start();
                        let guard = writer_buffer.write_bulk_wait(BLOCK_SIZE);
                        // Write interleaved samples to the ring buffer
                        guard.write_samples(&interleaved_samples);
                        generator_profiler.write_bulk.end();
                    }
                })
                .unwrap(),
        );

        let bus_update = self.update_bus.clone();
        let events_stop_signal = Arc::clone(&self.stop_signal);
        let events_bus = Arc::clone(&self.main_bus);
        let events_profiler = Arc::clone(&self.profiler);
        self.events_thread = Some(
            Builder::new()
                .name("aud_events".into())
                .spawn(move || {
                    loop {
                        events_profiler.events_tps.tick(1);

                        // Check if the stop signal is set
                        if events_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio events thread stopping");
                            break;
                        }

                        // Wait for the update bus to signal that there are new events
                        let (lock, cvar) = &*bus_update;
                        let mut update_count = lock.lock().unwrap();
                        while *update_count == 0 {
                            update_count = cvar.wait(update_count).unwrap();
                        }

                        // Reset the update count
                        *update_count = 0;

                        // Process events in the main bus
                        debug!("Processing events in the main bus");
                        let mut bus_guard = events_bus.lock().unwrap();
                        events_profiler.events.start();
                        bus_guard.process_events();
                        events_profiler.events.end();
                        drop(bus_guard);
                    }
                })
                .unwrap(),
        );

        let statistics_profiler = Arc::clone(&self.profiler);
        let statistics_stop_signal = Arc::clone(&self.stop_signal);
        self.statistics_thread = Some(
            Builder::new()
                .name("aud_stats".into())
                .spawn(move || {
                    loop {
                        statistics_profiler.process_tps.update();
                        statistics_profiler.events_tps.update();

                        // Check if the stop signal is set
                        if statistics_stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("Audio statistics thread stopping");
                            break;
                        }

                        let (proc_min, proc_av, proc_max) = statistics_profiler.process.get_stat();
                        let (proc_tps_min, proc_tps_av, proc_tps_max) =
                            statistics_profiler.process_tps.get_stat();
                        let (write_bulk_min, write_bulk_av, write_bulk_max) =
                            statistics_profiler.write_bulk.get_stat();
                        let (events_min, events_av, events_max) =
                            statistics_profiler.events.get_stat();
                        let (events_tps_min, events_tps_av, events_tps_max) =
                            statistics_profiler.events_tps.get_stat();
                        let (available_min, available_av, available_max) =
                            statistics_profiler.available_minmax.get_stat();

                        info!(
                            "Gen: {:.1}/{:.1} (of {:.1}) ({:.0}). Ev: {:.1} ({:.0}). Buffer: {:}/{:}/{:}",
                            proc_av,
                            write_bulk_av,
                            1000.0 / ((sample_rate as usize * (DEVICE_BUFFER_SIZE / BLOCK_SIZE)) / (DEVICE_BUFFER_SIZE)) as f32,
                            proc_tps_av,
                            events_av,
                            events_tps_av,
                            available_min,
                            available_av,
                            available_max
                        );

                        // Sleep for a short duration to avoid busy-waiting
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                })
                .unwrap(),
        );

        let reader_profiler = Arc::clone(&self.profiler);
        let reader_buffer = Arc::clone(&self.ring_buffer);
        self.backend
            .open(move |buffer: &mut InterleavedSampleBuffer<SampleType>| {
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
            })
            .map_err(DeviceOpenError::BackendSpecific)?;

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
        // Wait for the generator thread to finish
        if let Some(thread) = self.generator_thread.take() {
            if let Err(e) = thread.join() {
                warn!("Failed to join audio generator thread: {:?}", e);
            }
        }
        // Notify the events thread to stop
        *self.update_bus.0.lock().unwrap() = 1; // Set the update count to 1 to wake up the events thread
        self.update_bus.1.notify_all(); // Notify the events thread to wake up and exit

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
