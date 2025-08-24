use crate::backend::{
    InternalBackendConfig, PlayerBackend, PlayerBackendConfig, PlayerBackendError,
    PlayerBackendTrait,
};
use crate::dsp::detect_features;
use crate::entities::events::AudioEvent;
use crate::entities::sinks::InterleavedSink;
use crate::entities::Source;
use crate::sample::MappedInterleavedBuffer;
use crate::{ChannelsCount, SampleRate, SampleType, SamplesCount, BLOCK_SIZE, CHANNELS_COUNT};
use crossbeam_queue::ArrayQueue;
use dawn_ecs::Tick;
use evenio::component::Component;
use evenio::event::{GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use log::{debug, info, warn};
use std::fmt::{Display, Formatter};
use std::sync::{atomic::AtomicBool, Arc};
use std::thread::Builder;
use std::time::{Duration, Instant};
use dawn_util::profile::{Counter, MonitorSample, Stopwatch};

const EVENTS_QUEUE_CAPACITY: usize = 1024;
const MONITOR_QUEUE_CAPACITY: usize = 32;

/// Event sent every second with profiling data about the audio player.
#[derive(GlobalEvent)]
pub struct PlayerMonitoring {
    /// Number of ticks per second the renderer was called
    pub render_tps: MonitorSample<f32>,
    /// Number of events processed per second
    pub events_tps: MonitorSample<f32>,

    /// Time consumed by the renderer
    pub render: MonitorSample<Duration>,
    /// Time consumed by the event processing
    pub events: MonitorSample<Duration>,

    /// Average renderer load in percent
    pub load: MonitorSample<f32>,

    // Player parameters
    pub sample_rate: SampleRate,
    pub channels: ChannelsCount,
    pub block_size: SamplesCount,
}

trait PlayerMonitorTrait {
    fn set_queue(&mut self, _queue: Arc<ArrayQueue<PlayerMonitoring>>) {}
    fn events_start(&mut self) {}
    fn events_end(&mut self, _processed: usize) {}
    fn renderer_start(&mut self) {}
    fn renderer_end(&mut self) {}
}

struct PlayerMonitor {
    queue: Option<Arc<ArrayQueue<PlayerMonitoring>>>,
    last_update: Instant,
    sample_rate: SampleRate,
    renderer_time: Stopwatch,
    renderer_tps: Counter,
    events: Stopwatch,
    events_tps: Counter,
}

impl PlayerMonitor {
    fn new(sample_rate: SampleRate) -> Self {
        PlayerMonitor {
            queue: None,
            last_update: Instant::now(),
            sample_rate,
            renderer_time: Stopwatch::new(0.5),
            renderer_tps: Counter::new(Duration::from_secs(1), 0.5),
            events: Stopwatch::new(0.5),
            events_tps: Counter::new(Duration::from_secs(1), 0.5),
        }
    }
}

impl PlayerMonitorTrait for PlayerMonitor {
    fn set_queue(&mut self, queue: Arc<ArrayQueue<PlayerMonitoring>>) {
        self.queue = Some(queue);
    }

    fn events_start(&mut self) {
        self.renderer_tps.count(1);
        self.events.start();
    }

    fn events_end(&mut self, processed: usize) {
        self.events_tps.count(processed);
        self.events.stop();
    }

    fn renderer_start(&mut self) {
        self.renderer_time.start();
    }

    fn renderer_end(&mut self) {
        self.renderer_time.stop();

        // Call every second to send profiling data
        if self.last_update.elapsed().as_secs_f32() >= 1.0 {
            self.last_update = Instant::now();

            // Collect and log statistics
            self.renderer_tps.update();
            self.events_tps.update();

            if let Some(queue) = &self.queue {
                // Calculate the average load of the player
                // Number of samples that actually processed by one render call
                // (assuming that no underruns happens).
                let render_tps = self.renderer_tps.get();
                let renderer_time = self.renderer_time.get();
                let events_time = self.events.get();

                let total_time_average = renderer_time.average() + events_time.average();
                let total_time_min = renderer_time.min() + events_time.min();
                let total_time_max = renderer_time.max() + events_time.max();

                let av_actual_samples = self.sample_rate as f32 / render_tps.average();
                // Calculate the allowed time for one render call
                let allowed_time = av_actual_samples / self.sample_rate as f32 * 1000.0;
                let load = MonitorSample::new(
                    total_time_min.as_secs_f32() / allowed_time,
                    total_time_average.as_secs_f32() / allowed_time,
                    total_time_max.as_secs_f32() / allowed_time,
                );

                let frame = PlayerMonitoring {
                    render: renderer_time,
                    render_tps,
                    events: events_time,
                    events_tps: self.events_tps.get(),
                    load,
                    sample_rate: self.sample_rate,
                    channels: CHANNELS_COUNT,
                    block_size: BLOCK_SIZE,
                };

                // Send the monitoring frame to the queue
                if queue.push(frame).is_err() {
                    warn!("Cannot send monitoring frame");
                }
            }
        }
    }
}

struct DummyPlayerMonitor;

impl PlayerMonitorTrait for DummyPlayerMonitor {}

/// The audio player is a component that handles audio output and processing.
/// It is responsible for rendering audio from a source (like a music
/// track or sound effect) and sending it to the audio backend for playback.
#[derive(Component)]
pub struct Player {
    // The backend that handles audio output and most of the audio conversion.
    backend: PlayerBackend<SampleType>,
    // Event queue for processing audio events.
    events: Arc<ArrayQueue<AudioEvent>>,
    // Queue for transferring monitor frames to the main thread.
    monitor_queue: Arc<ArrayQueue<PlayerMonitoring>>,
}

impl Drop for Player {
    fn drop(&mut self) {
        info!("Dropping Player");
        self.backend.close().unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerError {
    InvalidSampleRate(SampleRate),
    InvalidChannels(ChannelsCount),
    InvalidBufferSize(SamplesCount),
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
    /// Creates a new audio player with the specified sample rate, backend
    /// configuration, sink (programmed audio renderer), and profiling option.
    /// Player is responsible for outputting audio to the OS-specific audio backend.
    /// If profiling is enabled, it will collect statistics about the audio
    /// processing, allowing you to analyze performance and behavior.
    pub fn new<S>(
        sample_rate: SampleRate,
        backend_config: PlayerBackendConfig,
        sink: InterleavedSink<S>,
        use_profiling: bool,
    ) -> Result<Self, PlayerError>
    where
        S: Source + Send + Sync + 'static,
    {
        if use_profiling {
            Self::new_inner(
                sample_rate,
                backend_config,
                sink,
                PlayerMonitor::new(sample_rate),
            )
        } else {
            Self::new_inner(sample_rate, backend_config, sink, DummyPlayerMonitor)
        }
    }

    fn new_inner<S, P>(
        sample_rate: SampleRate,
        backend_config: PlayerBackendConfig,
        mut sink: InterleavedSink<S>,
        mut monitor: P,
    ) -> Result<Self, PlayerError>
    where
        S: Source + Send + Sync + 'static,
        P: PlayerMonitorTrait + Send + Sync + 'static,
    {
        if sample_rate == 0 {
            return Err(PlayerError::InvalidSampleRate(sample_rate));
        }

        // Setup monitor
        let monitor_queue = Arc::new(ArrayQueue::<PlayerMonitoring>::new(MONITOR_QUEUE_CAPACITY));
        monitor.set_queue(Arc::clone(&monitor_queue));

        // Should not be here, since DSP processing is not required
        // for the player, but for convincing we will call it here.
        detect_features();

        // Create and start the audio backend
        let backend_config = InternalBackendConfig {
            backend_specific: backend_config,
            sample_rate,
            channels: CHANNELS_COUNT,
            buffer_size: BLOCK_SIZE,
        };
        let events_queue = Arc::new(ArrayQueue::<AudioEvent>::new(EVENTS_QUEUE_CAPACITY));
        let events_queue_clone = Arc::clone(&events_queue);
        let mut backend = PlayerBackend::<SampleType>::new(backend_config)
            .map_err(PlayerError::FailedToCreateBackend)?;
        backend
            .open(move |output: &mut MappedInterleavedBuffer<f32>| {
                // Process events from the queue
                monitor.events_start();
                let mut processed_events = 0;
                while let Some(event) = events_queue_clone.pop() {
                    // Process the event
                    sink.dispatch(&event);
                    processed_events += 1;
                }
                monitor.events_end(processed_events);

                // Render the audio output
                monitor.renderer_start();
                sink.render(output);
                monitor.renderer_end();
            })
            .map_err(PlayerError::FailedToStartBackend)?;

        Ok(Player {
            backend,
            events: events_queue,
            monitor_queue,
        })
    }

    /// Transfers the audio event to the sink for processing.
    /// The event will be processed at the start of the next audio block.
    /// Usually you want not to use this method directly.
    /// Instead, you should use the `AudioEvent` events in the ECS
    pub fn push_event(&self, event: &AudioEvent) {
        self.events.push(event.clone()).unwrap();
    }

    /// After attaching the player to the ECS, it will automatically consume audio events
    /// of type `AudioEvent` and pass them to the sink for processing.
    /// Also, if you enabled profiling, it will send profiling data
    /// as `PlayerMonitoring` events to the ECS every second.
    /// This function moves the player into the ECS world.
    pub fn attach_to_ecs(self, world: &mut World) {
        // Setup the audio player entity in the ECS
        let player_entity = world.spawn();
        world.insert(player_entity, self);

        fn audio_events_handler(r: Receiver<AudioEvent>, player: Single<&Player>) {
            // Remap the event to the player (usually run in the different thread)
            player.0.push_event(r.event);
        }

        fn tick_handler(
            _: Receiver<Tick>,
            player: Single<&Player>,
            mut sender: Sender<PlayerMonitoring>,
        ) {
            // Check if there's any monitor frame to process.
            // If so, push them to the ECS
            while let Some(frame) = player.0.monitor_queue.pop() {
                sender.send(frame);
            }
        }

        // Setup the audio events handler (from the ECS)
        world.add_handler(audio_events_handler.low());
        // Setup transfer of monitor frames to the ECS
        world.add_handler(tick_handler.low());
    }
}
