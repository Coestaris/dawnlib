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
use evenio::component::Component;
use evenio::event::{Event, GlobalEvent, Receiver, Sender};
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;
use log::{debug, info};
use std::fmt::{Display, Formatter};
use std::sync::{atomic::AtomicBool, Arc};
use std::thread::{Builder, JoinHandle};
use yage2_core::ecs::Tick;
use yage2_core::profile::{PeriodProfiler, ProfileFrame, TickProfiler};

const STATISTICS_THREAD_NAME: &str = "aud_stats";
const EVENTS_QUEUE_CAPACITY: usize = 1024;
const PROFILE_QUEUE_CAPACITY: usize = 32;

/// Event sent every second with profiling data about the audio player.
#[derive(GlobalEvent)]
pub struct PlayerProfileFrame {
    // Time consumed by the renderer (in milliseconds)
    pub render: ProfileFrame,
    // Number of ticks per second the renderer was called
    pub render_tps: ProfileFrame,
    // Time consumed by the event processing (in milliseconds)
    pub events: ProfileFrame,
    // Number of events processed per second
    pub events_tps: ProfileFrame,
    // Player parameters
    pub sample_rate: SampleRate,
    pub channels: ChannelsCount,
    pub block_size: SamplesCount,
}

trait PlayerProfilerTrait {
    fn events_start(&self) {}
    fn events_end(&self, _processed: usize) {}
    fn renderer_start(&self) {}
    fn renderer_end(&self) {}
    fn spawn_thread(
        self: Arc<Self>,
        _stop_signal: Arc<AtomicBool>,
        _sender: Arc<ArrayQueue<PlayerProfileFrame>>,
    ) -> Result<(), PlayerError> {
        Ok(())
    }
}

struct PlayerProfiler {
    sample_rate: SampleRate,
    renderer_time: PeriodProfiler,
    renderer_tps: TickProfiler,
    events: PeriodProfiler,
    events_tps: TickProfiler,
}

impl PlayerProfiler {
    fn new(sample_rate: SampleRate) -> Self {
        PlayerProfiler {
            sample_rate,
            renderer_time: PeriodProfiler::new(0.2),
            renderer_tps: TickProfiler::new(1.0),
            events: PeriodProfiler::new(0.2),
            events_tps: TickProfiler::new(1.0),
        }
    }
}

impl PlayerProfilerTrait for PlayerProfiler {
    fn events_start(&self) {
        self.renderer_tps.tick(1);
        self.events.start();
    }

    fn events_end(&self, processed: usize) {
        self.events_tps.tick(processed as u32);
        self.events.end();
    }

    fn renderer_start(&self) {
        self.renderer_time.start();
    }

    fn renderer_end(&self) {
        self.renderer_time.end();
    }

    fn spawn_thread(
        self: Arc<Self>,
        stop_signal: Arc<AtomicBool>,
        sender: Arc<ArrayQueue<PlayerProfileFrame>>,
    ) -> Result<(), PlayerError> {
        Builder::new()
            .name(STATISTICS_THREAD_NAME.into())
            .spawn(move || {
                loop {
                    // Check if the stop signal is set
                    if stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
                        debug!("Received stop signal");
                        break;
                    }

                    // Collect and log statistics
                    self.renderer_tps.update();
                    self.events_tps.update();

                    let frame = PlayerProfileFrame {
                        render: self.renderer_time.get_frame(),
                        render_tps: self.renderer_tps.get_frame(),
                        events: self.events.get_frame(),
                        events_tps: self.events_tps.get_frame(),
                        sample_rate: self.sample_rate,
                        channels: CHANNELS_COUNT,
                        block_size: BLOCK_SIZE,
                    };

                    // Send the profile frame to the queue
                    let _ = sender.push(frame);

                    // Sleep for a short duration to avoid busy-waiting
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            })
            .map_err(|_| PlayerError::ProfilerSetupFailed)?;
        Ok(())
    }
}

struct DummyPlayerProfiler;

impl PlayerProfilerTrait for DummyPlayerProfiler {}

/// The audio player is a component that handles audio output and processing.
/// It is responsible for rendering audio from a source (like a music
/// track or sound effect) and sending it to the audio backend for playback.
#[derive(Component)]
pub struct Player {
    // The backend that handles audio output and most of the audio conversion.
    backend: PlayerBackend<SampleType>,
    // A signal that is used to stop the audio processing thread.
    stop_signal: Arc<AtomicBool>,
    // Event queue for processing audio events.
    events: Arc<ArrayQueue<AudioEvent>>,
    // Queue for transferring profile frames to the main thread.
    profile_frames: Arc<ArrayQueue<PlayerProfileFrame>>,
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
    ProfilerSetupFailed,
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
            PlayerError::ProfilerSetupFailed => {
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
                PlayerProfiler::new(sample_rate),
            )
        } else {
            Self::new_inner(sample_rate, backend_config, sink, DummyPlayerProfiler)
        }
    }

    fn new_inner<S, P>(
        sample_rate: SampleRate,
        backend_config: PlayerBackendConfig,
        mut sink: InterleavedSink<S>,
        profiler: P,
    ) -> Result<Self, PlayerError>
    where
        S: Source + Send + Sync + 'static,
        P: PlayerProfilerTrait + Send + Sync + 'static,
    {
        if sample_rate == 0 {
            return Err(PlayerError::InvalidSampleRate(sample_rate));
        }

        // Setup profiler
        let stop_signal = Arc::new(AtomicBool::new(false));
        let profiler = Arc::new(profiler);
        let profiler_queue = Arc::new(ArrayQueue::<PlayerProfileFrame>::new(
            PROFILE_QUEUE_CAPACITY,
        ));
        profiler
            .clone()
            .spawn_thread(Arc::clone(&stop_signal), Arc::clone(&profiler_queue))
            .map_err(|_| PlayerError::ProfilerSetupFailed)?;

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
                profiler.events_start();
                let mut processed_events = 0;
                while let Some(event) = events_queue_clone.pop() {
                    // Process the event
                    sink.dispatch(&event);
                    processed_events += 1;
                }
                profiler.events_end(processed_events);

                // Render the audio output
                profiler.renderer_start();
                sink.render(output);
                profiler.renderer_end();
            })
            .map_err(PlayerError::FailedToStartBackend)?;

        Ok(Player {
            backend,
            stop_signal,
            events: events_queue,
            profile_frames: profiler_queue,
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
    /// as `PlayerProfileFrame` events to the ECS every second.
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
            mut sender: Sender<PlayerProfileFrame>,
        ) {
            // Check if there's any profile frame to process.
            // If so, push them to the ECS
            if let Some(frame) = player.0.profile_frames.pop() {
                sender.send(frame);
            }
        }

        // Setup the audio events handler (from the ECS)
        world.add_handler(audio_events_handler.low());
        // Setup transfer of profile frames to the ECS
        world.add_handler(tick_handler.low());
    }
}
