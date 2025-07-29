use crate::backend::{
    InternalBackendConfig, PlayerBackend, PlayerBackendConfig, PlayerBackendError,
    PlayerBackendTrait,
};
use crate::dsp::detect_features;
use crate::entities::events::EventBox;
use crate::entities::sinks::InterleavedSink;
use crate::entities::Source;
use crate::sample::MappedInterleavedBuffer;
use crate::{ChannelsCount, SampleRate, SampleType, SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};
use crossbeam_queue::ArrayQueue;
use log::{debug, info, warn};
use std::fmt::{Display, Formatter};
use std::sync::{atomic::AtomicBool, Arc};
use yage2_core::profile::{PeriodProfiler, TickProfiler};
use yage2_core::threads::{ThreadManager, ThreadPriority};

const STATISTICS_THREAD_NAME: &str = "aud_stats";
const STATISTICS_THREAD_PRIORITY: ThreadPriority = ThreadPriority::Low;
const EVENTS_QUEUE_CAPACITY: usize = 1024;

pub struct PlayerConfig<'tm, 'scope, 'env, F>
where
    'env: 'scope,
    F: FnMut(&ProfileFrame) + Send + Sync + 'static,
{
    /// Sample rate of the audio stream
    pub sample_rate: SampleRate,
    /// Backend-specific configuration used for fine-tuning the audio backend
    pub backend_config: PlayerBackendConfig,
    /// Scoped thread manager that will be used to spawn the statistics thread
    pub thread_manager: &'tm ThreadManager<'scope, 'env>,
    /// Optional profiler handler that will be called with the profiling data
    pub profiler: Option<F>,
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

pub struct Player {
    // The backend that handles audio output and most of the audio conversion.
    backend: PlayerBackend<SampleType>,
    // A signal that is used to stop the audio processing thread.
    stop_signal: Arc<AtomicBool>,
    // Event queue for processing audio events.
    events: Arc<ArrayQueue<EventBox>>,
}

impl Drop for Player {
    fn drop(&mut self) {
        info!("Dropping Player");
        self.backend.close().unwrap();

        // Notify the generator thread to stop
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerError {
    InvalidSampleRate(SampleRate),
    InvalidChannels(ChannelsCount),
    InvalidBufferSize(SamplesCount),
    FailedToSpawnStatisticsThread,
    FailedToStartBackend(PlayerBackendError),
    FailedToCreateBackend(PlayerBackendError),
}

impl Display for PlayerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerError::InvalidSampleRate(rate) => {
                write!(f, "Invalid sample rate: {}", rate)
            }
            PlayerError::InvalidChannels(channels) => {
                write!(f, "Invalid number of channels: {}", channels)
            }
            PlayerError::InvalidBufferSize(size) => {
                write!(f, "Invalid buffer size: {}", size)
            }
            PlayerError::FailedToSpawnStatisticsThread => {
                write!(f, "Failed to spawn statistics thread")
            }
            PlayerError::FailedToStartBackend(err) => {
                write!(f, "Failed to start backend: {}", err)
            }
            PlayerError::FailedToCreateBackend(err) => {
                write!(f, "Failed to create backend: {}", err)
            }
        }
    }
}

impl Player {
    pub fn new<S, F>(
        config: PlayerConfig<F>,
        mut sink: InterleavedSink<S>,
    ) -> Result<Self, PlayerError>
    where
        S: Source + Send + Sync + 'static,
        F: FnMut(&ProfileFrame) + Send + Sync + 'static,
    {
        if config.sample_rate == 0 {
            return Err(PlayerError::InvalidSampleRate(config.sample_rate));
        }

        // If requested, start the statistics thread (for profiling)
        let stop_signal = Arc::new(AtomicBool::new(false));
        let profilers = Arc::new(Profilers::default());
        match config.profiler {
            Some(handler) => {
                Player::spawn_statistics_thread(
                    config.thread_manager,
                    handler,
                    Arc::clone(&stop_signal),
                    config.sample_rate,
                    Arc::clone(&profilers),
                )?;
            }
            None => {
                info!("Profiler handler is not set, statistics thread will not be spawned");
            }
        }

        // Should not be here, since DSP processing is not required
        // for the player, but for convincing we will call it here.
        detect_features();

        // Create and start the audio backend
        let backend_config = InternalBackendConfig {
            backend_specific: config.backend_config,
            sample_rate: config.sample_rate,
            channels: CHANNELS_COUNT,
            buffer_size: BLOCK_SIZE,
        };
        let events_queue = Arc::new(ArrayQueue::<EventBox>::new(EVENTS_QUEUE_CAPACITY));
        let events_queue_clone = Arc::clone(&events_queue);
        let mut backend = PlayerBackend::<SampleType>::new(backend_config)
            .map_err(PlayerError::FailedToCreateBackend)?;
        backend
            .open(move |output: &mut MappedInterleavedBuffer<f32>| {
                profilers.renderer_tps.tick(1);

                // Process events from the queue
                profilers.events.start();
                let mut processed_events = 0;
                while let Some(event) = events_queue_clone.pop() {
                    // Process the event
                    sink.dispatch(&event);
                    processed_events += 1;
                }
                profilers.events_tps.tick(processed_events);
                profilers.events.end();

                // Render the audio output
                profilers.renderer_time.start();
                sink.render(output);
                profilers.renderer_time.end();
            })
            .map_err(PlayerError::FailedToStartBackend)?;

        Ok(Player {
            backend,
            stop_signal,
            events: events_queue,
        })
    }

    pub fn push_event(&self, event: &EventBox) {
        self.events.push(event.clone()).unwrap();
    }

    fn spawn_statistics_thread<F>(
        tm: &ThreadManager,
        mut handler: F,
        stop_signal: Arc<AtomicBool>,
        sample_rate: SampleRate,
        profilers: Arc<Profilers>,
    ) -> Result<(), PlayerError>
    where
        F: FnMut(&ProfileFrame) + Send + Sync + 'static,
    {
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
                    profilers.renderer_tps.update();
                    profilers.events_tps.update();

                    let (render_min, render_av, render_max) = profilers.renderer_time.get_stat();
                    let (render_tps_min, render_tps_av, render_tps_max) =
                        profilers.renderer_tps.get_stat();
                    let (events_min, events_av, events_max) = profilers.events.get_stat();
                    let (events_tps_min, events_tps_av, events_tps_max) =
                        profilers.events_tps.get_stat();

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

                    handler(&frame);

                    // Sleep for a short duration to avoid busy-waiting
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            },
        )
        .map_err(|_| PlayerError::FailedToSpawnStatisticsThread)
    }
}
