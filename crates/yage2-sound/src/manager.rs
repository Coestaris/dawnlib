use crate::backend::{
    AudioBackend, BackendDeviceTrait, BackendSpecificConfig, CreateBackendConfig,
};
use crate::dsp::detect_features;
use crate::entities::bus::Bus;
use crate::entities::sinks::InterleavedSink;
use crate::entities::{BlockInfo, Effect, NodeRef, Source};
use crate::error::{AudioManagerCreationError, AudioManagerStartError, AudioManagerStopError};
use crate::sample::{
    InterleavedBlock, InterleavedSample, MappedInterleavedBuffer, PlanarBlock, Sample,
};
use crate::{ChannelsCount, SampleRate, SampleType, SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};
use log::{debug, info, warn};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use yage2_core::profile::{MinMaxProfiler, PeriodProfiler, TickProfiler};
use yage2_core::resources::{ResourceManager, ResourceType};
use yage2_core::threads::{ThreadManager, ThreadPriority};

const STATISTICS_THREAD_NAME: &str = "aud_stats";
const STATISTICS_THREAD_PRIORITY: ThreadPriority = ThreadPriority::Low;

pub struct AudioManagerConfig<'tm, 'scope, 'env>
where
    'env: 'scope,
{
    pub thread_manager: &'tm ThreadManager<'scope, 'env>,
    pub resource_manager: Arc<ResourceManager>,

    pub backend_specific: BackendSpecificConfig,
    pub sample_rate: SampleRate,

    pub profiler_handler: Option<fn(&ProfileFrame)>,
}

struct Profilers {
    renderer_time: PeriodProfiler,
    renderer_tps: TickProfiler,
    events: PeriodProfiler,
    events_tps: TickProfiler,
}

impl Default for Profilers {
    fn default() -> Self {
        Profilers {
            renderer_time: PeriodProfiler::new(0.2),
            renderer_tps: TickProfiler::new(1.0),
            events: PeriodProfiler::new(0.2),
            events_tps: TickProfiler::new(1.0),
        }
    }
}

pub struct ProfileFrame {
    // Time consumed by the renderer (in milliseconds)
    pub render_min: f32,
    pub render_av: f32,
    pub render_max: f32,

    // Number of ticks per second the renderer was called
    pub render_tps_min: f32,
    pub render_tps_av: f32,
    pub render_tps_max: f32,

    // Time consumed by the event processing (in milliseconds)
    pub events_min: f32,
    pub events_av: f32,
    pub events_max: f32,

    // Number of events processed per second
    pub events_tps_min: f32,
    pub events_tps_av: f32,
    pub events_tps_max: f32,

    // Manager parameters
    pub sample_rate: SampleRate,
    pub channels: ChannelsCount,
    pub block_size: SamplesCount,
}

pub struct AudioManager {
    // The backend that handles audio output
    backend: AudioBackend<SampleType>,
    // Threads for audio generation, event processing, and statistics
    stop_signal: Arc<AtomicBool>,
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        info!("Closing audio manager");
        self.backend.close().unwrap();

        // Notify the generator thread to stop
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl AudioManager {
    pub fn new<S>(
        config: AudioManagerConfig,
        sink: InterleavedSink<S>,
    ) -> Result<Self, AudioManagerCreationError>
    where
        S: Source + Send + Sync + 'static,
    {
        if config.sample_rate == 0 {
            return Err(AudioManagerCreationError::InvalidSampleRate(
                config.sample_rate,
            ));
        }

        let backend_config = CreateBackendConfig {
            backend_specific: config.backend_specific,
            sample_rate: config.sample_rate,
            channels: CHANNELS_COUNT,
            buffer_size: BLOCK_SIZE,
        };

        let mut backend = AudioBackend::<SampleType>::new(backend_config)
            .map_err(AudioManagerCreationError::BackendSpecific)?;

        detect_features();

        #[cfg(feature = "resources-wav")]
        config.resource_manager.register_factory(
            ResourceType::AudioWAV,
            Arc::new(crate::resources::wav::WAVResourceFactory::new(
                config.sample_rate,
            )),
        );
        #[cfg(feature = "resources-ogg")]
        config.resource_manager.register_factory(
            ResourceType::AudioOGG,
            Arc::new(crate::resources::ogg::OGGResourceFactory::new(
                config.sample_rate,
            )),
        );
        #[cfg(feature = "resources-flac")]
        config.resource_manager.register_factory(
            ResourceType::AudioFLAC,
            Arc::new(crate::resources::flac::FLACResourceFactory::new(
                config.sample_rate,
            )),
        );

        let stop_signal = Arc::new(AtomicBool::new(false));
        let profilers = Arc::new(Profilers::default());

        AudioManager::spawn_statistics_thread(
            config.thread_manager,
            config.profiler_handler,
            Arc::clone(&stop_signal),
            config.sample_rate,
            Arc::clone(&profilers),
        )
        .unwrap();

        AudioManager::spawn_renderer(&mut backend, sink, Arc::clone(&profilers)).unwrap();

        Ok(AudioManager {
            backend,
            stop_signal,
        })
    }

    fn spawn_statistics_thread(
        tm: &ThreadManager,
        profiler_handler: Option<fn(&ProfileFrame)>,
        stop_signal: Arc<AtomicBool>,
        sample_rate: SampleRate,
        profilers: Arc<Profilers>,
    ) -> Result<(), AudioManagerStartError> {
        let profiler_handler = match &profiler_handler {
            Some(handler) => handler.clone(),
            None => {
                warn!("No profiler handler provided, statistics thread will not log data");
                return Ok(());
            }
        };

        let tick = move || {
            profilers.renderer_tps.update();
            profilers.events_tps.update();

            let (render_min, render_av, render_max) = profilers.renderer_time.get_stat();
            let (render_tps_min, render_tps_av, render_tps_max) = profilers.renderer_tps.get_stat();
            let (events_min, events_av, events_max) = profilers.events.get_stat();
            let (events_tps_min, events_tps_av, events_tps_max) = profilers.events_tps.get_stat();

            let frame = ProfileFrame {
                render_min,
                render_av,
                render_max,
                render_tps_min,
                render_tps_av,
                render_tps_max,
                events_min,
                events_av,
                events_max,
                events_tps_min,
                events_tps_av,
                events_tps_max,
                sample_rate,
                channels: CHANNELS_COUNT,
                block_size: BLOCK_SIZE,
            };

            profiler_handler(&frame);

            // Sleep for a short duration to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(1000));
        };

        tm.spawn(
            STATISTICS_THREAD_NAME.into(),
            STATISTICS_THREAD_PRIORITY,
            move || {
                loop {
                    // Check if the stop signal is set
                    if stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                        debug!("Received stop signal");
                        break;
                    }

                    // Collect and log statistics
                    tick();
                }
            },
        )
        .map_err(AudioManagerStartError::StatisticsThreadSpawnError)?;

        Ok(())
    }

    fn spawn_renderer<S>(
        backend: &mut AudioBackend<SampleType>,
        mut sink: InterleavedSink<S>,
        profilers: Arc<Profilers>,
    ) -> Result<(), AudioManagerStartError>
    where
        S: Source + Send + Sync + 'static,
    {
        let raw_fn = {
            move |output: &mut MappedInterleavedBuffer<f32>| {
                profilers.renderer_tps.tick(1);

                // TODO: Collect new event boxes
                profilers.events.start();
                let boxes = [];
                profilers.events_tps.tick(boxes.len() as u32);
                sink.dispatch(&boxes);
                profilers.events.end();

                profilers.renderer_time.start();
                sink.render(output);
                profilers.renderer_time.end();
            }
        };

        backend
            .open(raw_fn)
            .map_err(AudioManagerStartError::BackendSpecific)?;

        Ok(())
    }
}
