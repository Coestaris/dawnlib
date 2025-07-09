use crate::event::Event;
use crate::input::InputManager;
use crate::object::{DispatchAction, ObjectCtx, Renderable};
use crate::object_collection::ObjectsCollection;
use crate::view::{TickResult, View, ViewConfig, ViewError, ViewTrait};
use crate::vulkan::graphics::{Graphics, GraphicsConfig};
use crate::vulkan::GraphicsError;
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use yage2_core::profile::{PeriodProfiler, TickProfiler};
use yage2_core::sync::Rendezvous;
use yage2_core::time::current_us;

#[derive(Debug)]
#[allow(dead_code)]
pub enum RendererThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    ThreadJoinError,

    WindowCreationError(ViewError),
    WindowTickError(ViewError),
    GraphicsTickError(GraphicsError),
    GraphicsCreateError(GraphicsError),
    TheadPanic,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum LogicThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    TheadPanic,
    ThreadJoinError,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum StatisticsThreadError {
    AlreadyRunning,
    ThreadSpawnFailed,
    TheadPanic,
    ThreadJoinError,
}

#[derive(Default)]
pub(crate) struct RendererProfiler {
    pub fps: TickProfiler,
    pub renderables: TickProfiler,
    pub drawn_triangles: TickProfiler,

    pub before_frame_sync: PeriodProfiler,
    pub window_tick: PeriodProfiler,
    pub graphics_tick: PeriodProfiler,
    pub after_frame_sync: PeriodProfiler,
    pub copy_objects: PeriodProfiler,
}

#[derive(Default)]
pub(crate) struct LogicProfiler {
    pub eps: Arc<TickProfiler>,
    pub ups: TickProfiler,
    pub before_frame_sync: PeriodProfiler,
    pub logic_processing: PeriodProfiler,
    pub after_frame_sync: PeriodProfiler,
}

/// Insert passed statement only if the debug feature is enabled
macro_rules! profile {
    ($statement:expr) => {
        #[cfg(debug_assertions)]
        {
            $statement;
        }
    };
    ($statement:expr, $($args:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $statement($($args)*);
        }
    };
}

#[derive(Clone)]
pub(crate) struct RendererThreadConfig {
    pub view_config: ViewConfig,
    pub graphics_config: GraphicsConfig,

    // Synchronization points for the renderer thread
    // to synchronize with the logic thread
    pub before_frame: Arc<Rendezvous>,
    pub after_frame: Arc<Rendezvous>,

    // List of renderables that will be rendered by the renderer thread.
    pub renderer_objects: Arc<Mutex<Vec<Renderable>>>,

    // Shared data structure that will be used to share data between the
    // renderer and logic threads. After end of the each frame,
    // the renderer thread will copy the renderable objects into own list.
    pub logic_objects: Arc<Mutex<ObjectsCollection>>,

    pub stop_signal: Arc<AtomicBool>,

    // Debug profiler for the renderer thread
    pub profiler: Arc<RendererProfiler>,
}

/// This function creates a new thread that handles rendering operations.
/// It synchronizes with the logic thread using the provided rendezvous points.
/// It processes window events, renders the scene, and updates the renderable objects.
/// It returns a `JoinHandle` that can be used to join the thread later.
fn renderer_inner(
    cfg: RendererThreadConfig,
    events_sender: Sender<Event>,
) -> Result<(), RendererThreadError> {
    let mut view = View::open(cfg.view_config, events_sender)
        .map_err(RendererThreadError::WindowCreationError)?;

    let mut graphics = Graphics::open(
        cfg.graphics_config,
        |entry: &ash::Entry, instance: &ash::Instance| view.create_surface(entry, instance),
    )
    .map_err(RendererThreadError::GraphicsCreateError)?;

    info!("Renderer thread started");
    let mut result: Result<(), RendererThreadError> = Ok(());
    while !cfg.stop_signal.load(Ordering::Relaxed) {
        profile!(cfg.profiler.fps.tick(1));

        // Stage 1: Synchronize with the logic thread
        profile!(cfg.profiler.before_frame_sync.start());
        if !cfg.before_frame.wait() {
            warn!("Thread pre-frame synchronization failed, stopping renderer thread");
            break;
        }
        profile!(cfg.profiler.before_frame_sync.end());

        // Stage 2: Process window events and OS-specific stuff
        profile!(cfg.profiler.window_tick.start());
        match view.tick() {
            TickResult::Continue => {
                // Window tick was successful, continue processing
            }
            TickResult::Closed => {
                // Window tick returned false, which means the window was closed
                info!("Window closed, stopping renderer thread");
                break;
            }
            TickResult::Failed(e) => {
                // An error occurred during the window tick
                warn!("Window tick error: {:?}", e);
                result = Err(RendererThreadError::WindowTickError(e));
                break;
            }
        }

        profile!(cfg.profiler.window_tick.end());

        // Stage 3: Render the scene
        profile!(cfg.profiler.graphics_tick.start());
        let renderables = cfg.renderer_objects.lock().unwrap();
        match graphics.tick(renderables.as_slice()) {
            Ok(result) => {
                // Rendering was successful, update the profiler
                profile!(cfg.profiler.drawn_triangles.tick(result.drawn_triangles));
            }
            Err(e) => {
                // An error occurred during rendering
                warn!("Graphics tick error: {:?}", e);
                result = Err(RendererThreadError::GraphicsTickError(e));
                break;
            }
        }
        drop(renderables);
        profile!(cfg.profiler.graphics_tick.end());

        // Stage 4: Synchronize with the logic thread
        profile!(cfg.profiler.after_frame_sync.start());
        if !cfg.after_frame.wait() {
            warn!("Thread post-frame synchronization failed, stopping renderer thread");
            break;
        }
        profile!(cfg.profiler.after_frame_sync.end());

        // Stage 5: Copy renderable objects data
        profile!(cfg.profiler.copy_objects.start());
        let mut objects_collection = cfg.logic_objects.lock().unwrap();
        match objects_collection.updated_renderables() {
            Some(updated) => {
                profile!(cfg.profiler.renderables.tick(1));
                let mut renderables = cfg.renderer_objects.lock().unwrap();
                renderables.clear();
                for val in updated.values() {
                    renderables.push(val.clone());
                }
                drop(renderables);
            }
            None => {
                // No renderables updated, nothing to do
            }
        }
        drop(objects_collection);
        profile!(cfg.profiler.copy_objects.end());
    }

    cfg.stop_signal.store(true, Ordering::Relaxed);
    result
}

pub(crate) fn renderer(
    cfg: RendererThreadConfig,
    events_sender: Sender<Event>,
) -> Result<(), RendererThreadError> {
    let cfg_clone = cfg.clone();
    let res = std::panic::catch_unwind(|| renderer_inner(cfg_clone, events_sender)).unwrap_or_else(
        |_| {
            warn!("Renderer thread panicked, stopping thread");
            Err(RendererThreadError::TheadPanic)
        },
    );

    cfg.stop_signal.store(true, Ordering::Relaxed);
    cfg.before_frame.unlock();
    cfg.after_frame.unlock();
    res
}

#[derive(Clone)]
pub struct LogicThreadConfig {
    // Synchronization points for the renderer thread
    // to synchronize with the logic thread
    pub before_frame: Arc<Rendezvous>,
    pub after_frame: Arc<Rendezvous>,

    // Shared data structure that will be used to share data between the
    // renderer and logic threads. After end of the each frame,
    // the renderer thread will copy the renderable objects into own list.
    pub logic_objects: Arc<Mutex<ObjectsCollection>>,

    // Shared data structure that will be used to share data between the
    // renderer and logic threads. It contains a stop signal that can be used
    // to stop the threads gracefully.
    pub stop_signal: Arc<AtomicBool>,

    // Debug profiler for the logic thread
    pub profiler: Arc<LogicProfiler>,
}

/// This function creates a new thread that handles the game logic.
/// It synchronizes with the renderer thread using the provided rendezvous points.
/// It processes input events, updates game logic, and dispatches events to the objects.
/// It returns a `JoinHandle` that can be used to join the thread later.
fn logic_inner(
    cfg: LogicThreadConfig,
    events_receiver: Receiver<Event>,
) -> Result<(), LogicThreadError> {
    info!("Logic thread started");
    let mut input_manager = InputManager::new(events_receiver, cfg.profiler.eps.clone());
    let mut first_tick = true;
    let mut prev_tick = 0;
    while !cfg.stop_signal.load(Ordering::Relaxed) {
        profile!(cfg.profiler.ups.tick(1));

        // Stage 1: Synchronize with the renderer thread
        profile!(cfg.profiler.before_frame_sync.start());
        if !cfg.before_frame.wait() {
            warn!("Thread pre-frame synchronization failed, stopping logic thread");
            break;
        }
        profile!(cfg.profiler.before_frame_sync.end());

        // Stage 2: Process logic
        profile!(cfg.profiler.logic_processing.start());
        let mut objects_collection = cfg.logic_objects.lock().unwrap();

        // Stage 2.1: Process input events
        let mut events = input_manager.poll_events();
        for event in events.iter() {
            input_manager.on_event(event);
        }

        // Stage 2.2: Dispatch events to objects
        let current_us = current_us();
        let time_delta = (current_us - prev_tick) as f32 / 1000.0;
        prev_tick = current_us;
        let object_ctx = ObjectCtx {
            input_manager: &input_manager,
        };
        if first_tick {
            // On the first tick, we need to initialize the objects
            // and dispatch the Create event to them.
            events.push(Event::Create);
            first_tick = false;
        }
        events.push(Event::Update(time_delta));
        for event in events {
            let actions = objects_collection.dispatch_event(&object_ctx, &event);
            for action in actions.iter() {
                match action {
                    DispatchAction::QuitApplication => {
                        info!("Quit application event received, stopping logic thread");
                        cfg.stop_signal.store(true, Ordering::Relaxed);
                        break;
                    }

                    // The majority of actions are handled in the object collection
                    // and do not require any special handling here.
                    _ => {
                        // No special handling needed for other actions
                    }
                }
            }
        }
        // Make sure that mutex is unlocked before the next synchronization point
        drop(objects_collection);
        profile!(cfg.profiler.logic_processing.end());

        // Stage 3: Synchronize with the renderer thread
        profile!(cfg.profiler.after_frame_sync.start());
        if !cfg.after_frame.wait() {
            warn!("Thread post-frame synchronization failed, stopping logic thread");
            break;
        }
        profile!(cfg.profiler.after_frame_sync.end());
    }

    Ok(())
}

pub(crate) fn logic(
    cfg: LogicThreadConfig,
    events_receiver: Receiver<Event>,
) -> Result<(), LogicThreadError> {
    let cfg_clone = cfg.clone();
    let res =
        std::panic::catch_unwind(|| logic_inner(cfg_clone, events_receiver)).unwrap_or_else(|_| {
            warn!("Logic thread panicked, stopping thread");
            Err(LogicThreadError::TheadPanic)
        });

    cfg.stop_signal.store(true, Ordering::Relaxed);
    cfg.before_frame.unlock();
    cfg.after_frame.unlock();
    res
}

#[derive(Clone)]
pub struct StatisticsThreadConfig {
    pub renderer_profiler: Arc<RendererProfiler>,
    pub logic_profiler: Arc<LogicProfiler>,
    pub stop_signal: Arc<AtomicBool>,
    pub logic_objects: Arc<Mutex<ObjectsCollection>>,
    pub renderer_objects: Arc<Mutex<Vec<Renderable>>>,
}

/// This function creates a new thread that handles statistics gathering.
/// It runs only in debug mode and collects various performance metrics
/// such as FPS, UPS, number of renderables, drawn triangles, and event processing speed.
/// It returns a `JoinHandle` that can be used to join the thread later.
#[cfg(debug_assertions)]
fn statistics_inner(cfg: StatisticsThreadConfig) -> Result<(), StatisticsThreadError> {
    info!("Statistics thread started");

    cfg.logic_profiler.ups.reset();
    cfg.logic_profiler.eps.reset();
    cfg.renderer_profiler.fps.reset();
    cfg.renderer_profiler.renderables.reset();
    cfg.renderer_profiler.drawn_triangles.reset();

    let mut reset_counter = 0;
    while !cfg.stop_signal.load(Ordering::Relaxed) {
        cfg.renderer_profiler.fps.update();
        cfg.renderer_profiler.renderables.update();
        cfg.renderer_profiler.drawn_triangles.update();
        cfg.logic_profiler.ups.update();
        cfg.logic_profiler.eps.update();

        if reset_counter % 10 == 0 {
            cfg.logic_profiler.ups.reset();
            cfg.logic_profiler.eps.reset();
            cfg.renderer_profiler.fps.reset();
            cfg.renderer_profiler.renderables.reset();
            cfg.renderer_profiler.drawn_triangles.reset();
        }
        reset_counter += 1;

        let (min_fps, fps, max_fps) = cfg.renderer_profiler.fps.get_stat();
        let (min_ups, ups, max_ups) = cfg.logic_profiler.ups.get_stat();
        let (_, r_renderables_avg, _) = cfg.renderer_profiler.renderables.get_stat();

        let (_, r_bf_avg, _) = cfg.renderer_profiler.before_frame_sync.get_stat();
        let (_, r_wt_avg, _) = cfg.renderer_profiler.window_tick.get_stat();
        let (_, r_gt_avg, _) = cfg.renderer_profiler.graphics_tick.get_stat();
        let (_, r_af_avg, _) = cfg.renderer_profiler.after_frame_sync.get_stat();
        let (_, r_co_avg, _) = cfg.renderer_profiler.copy_objects.get_stat();

        let (_, l_bf_avg, _) = cfg.logic_profiler.before_frame_sync.get_stat();
        let (_, l_lp_avg, _) = cfg.logic_profiler.logic_processing.get_stat();
        let (_, l_af_avg, _) = cfg.logic_profiler.after_frame_sync.get_stat();

        let (_, eps_avg, _) = cfg.logic_profiler.eps.get_stat();

        let objects = cfg.logic_objects.lock().unwrap();
        let num_objects = objects.alive_objects().len();
        drop(objects);
        let renderables = cfg.renderer_objects.lock().unwrap();
        let num_renderables = renderables.len();
        drop(renderables);

        info!(
                        "Obj: {:}, Ev: {:2.0}, R: {:}, FPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1} {:.1} {:.1} {:.0}], UPS: {:.1}/{:.1}/{:.1} [{:.1} {:.1} {:.1}]",
                        num_objects,
                        eps_avg,
                        num_renderables,
                        min_fps,
                        fps,
                        max_fps,
                        r_bf_avg,
                        r_wt_avg,
                        r_gt_avg,
                        r_af_avg,
                        r_co_avg,
                        r_renderables_avg,
                        min_ups,
                        ups,
                        max_ups,
                        l_bf_avg,
                        l_lp_avg,
                        l_af_avg,
                    );

        thread::sleep(std::time::Duration::from_millis(1000));
    }

    info!("Statistics thread stopping gracefully");
    cfg.stop_signal.store(true, Ordering::Relaxed);
    Ok(())
}

pub(crate) fn statistics(cfg: StatisticsThreadConfig) -> Result<(), StatisticsThreadError> {
    let cfg_clone = cfg.clone();
    let res = std::panic::catch_unwind(|| statistics_inner(cfg_clone)).unwrap_or_else(|_| {
        warn!("Statistics thread panicked, stopping thread");
        Err(StatisticsThreadError::TheadPanic)
    });

    cfg.stop_signal.store(true, Ordering::Relaxed);
    res
}
