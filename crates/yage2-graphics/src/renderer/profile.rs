use crate::passes::result::PassExecuteResult;
use crate::passes::MAX_RENDER_PASSES;
use crossbeam_queue::ArrayQueue;
use evenio::event::GlobalEvent;
use log::{debug, warn};
use std::collections::HashMap;
use std::panic::UnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use yage2_core::profile::{PeriodProfiler, ProfileFrame, TickProfiler};

#[derive(GlobalEvent)]
pub struct RendererProfileFrame {
    /// Actual number of frames drawn per second.
    pub fps: ProfileFrame,

    /// The time spend on the processing of the view tick
    /// (OS-dependent Window handling).
    pub view_tick: ProfileFrame,

    /// The time spend on processing the events from the ECS
    /// by the renderer pipeline.
    pub events: ProfileFrame,

    /// The time spend no the actual rendering of the frame
    /// (including the time spend on the backend).
    pub render: ProfileFrame,

    /// The approximate number of primitives drawn
    /// (triangles, lines, points, etc.) in the frame.
    pub drawn_primitives: ProfileFrame,

    /// The number of draw calls made in the frame.
    /// This is the number of times the GPU was instructed
    pub draw_calls: ProfileFrame,

    /// The amount of time spend on each render pass.
    /// The key is the name of the pass, and the value is the time spent on it.
    /// This is used for profiling the render passes.
    pub pass_profile: HashMap<String, ProfileFrame>,
}

pub(crate) trait RendererProfilerTrait: Send + Sync + 'static + UnwindSafe {
    fn set_queue(&mut self, _queue: Arc<ArrayQueue<RendererProfileFrame>>) {}
    fn set_pass_names(&mut self, _names: &[&str]) {}
    fn view_tick_start(&mut self) {}
    fn view_tick_end(&mut self) {}
    fn evens_start(&mut self) {}
    fn evens_end(&mut self) {}
    fn render_start(&mut self) {}
    fn render_end(
        &mut self,
        _execute_result: PassExecuteResult,
        _prof: &[Duration; MAX_RENDER_PASSES],
    ) {
    }
}

pub(crate) struct RendererProfiler {
    fps: TickProfiler,
    view_tick: PeriodProfiler,
    events: PeriodProfiler,
    render: PeriodProfiler,
    draw_calls: TickProfiler,
    drawn_primitives: TickProfiler,
    queue: Option<Arc<ArrayQueue<RendererProfileFrame>>>,
    pass_names: Vec<String>,
    last_send: std::time::Instant,
}

impl RendererProfilerTrait for RendererProfiler {
    fn set_queue(&mut self, queue: Arc<ArrayQueue<RendererProfileFrame>>) {
        // TODO: Implement thread-unsafe profiling
        self.queue = Some(queue);
    }

    fn set_pass_names(&mut self, names: &[&str]) {
        self.pass_names.clear();
        for name in names {
            self.pass_names.push(name.to_string());
        }
        debug!("Renderer profiler pass names set: {:?}", self.pass_names);
    }

    fn view_tick_start(&mut self) {
        self.fps.tick(1);
        self.view_tick.start();
    }

    fn view_tick_end(&mut self) {
        self.view_tick.end();
    }

    fn evens_start(&mut self) {
        self.events.start();
    }

    fn evens_end(&mut self) {
        self.events.end();
    }

    fn render_start(&mut self) {
        self.render.start();
    }

    fn render_end(
        &mut self,
        execute_result: PassExecuteResult,
        prof: &[Duration; MAX_RENDER_PASSES],
    ) {
        self.render.end();

        if let PassExecuteResult::Ok { primitives, calls } = execute_result {
            // Update the profiler with the number of primitives and draw calls
            self.drawn_primitives.tick(primitives as u32);
            self.draw_calls.tick(calls as u32);
        }

        // Call these each second
        let now = std::time::Instant::now();
        if now.duration_since(self.last_send) >= Duration::from_secs(1) {
            self.last_send = now;

            self.fps.update();
            self.drawn_primitives.update();

            let mut pass_profile = HashMap::with_capacity(self.pass_names.len());
            for (i, name) in self.pass_names.iter().enumerate() {
                if i < prof.len() {
                    let ms = prof[i].as_millis() as f32;
                    // TODO: Implement some kind of smoothing
                    pass_profile.insert(name.clone(), ProfileFrame::new(ms, ms, ms));
                }
            }
            let frame = RendererProfileFrame {
                fps: self.fps.get_frame(),
                view_tick: self.view_tick.get_frame(),
                events: self.events.get_frame(),
                render: self.render.get_frame(),
                drawn_primitives: self.drawn_primitives.get_frame(),
                draw_calls: self.draw_calls.get_frame(),
                pass_profile,
            };

            if let Some(queue) = &self.queue {
                if queue.push(frame).is_err() {
                    warn!("Renderer profiler queue is full, dropping frame");
                }
            } else {
                warn!("Renderer profiler queue is not set, dropping frame");
            }
        }
    }
}
impl RendererProfiler {
    pub fn new() -> Self {
        RendererProfiler {
            fps: TickProfiler::new(0.5),
            view_tick: PeriodProfiler::new(0.5),
            events: PeriodProfiler::new(0.5),
            render: PeriodProfiler::new(0.5),
            drawn_primitives: TickProfiler::new(1.0),
            queue: None,
            draw_calls: TickProfiler::new(1.0),
            last_send: std::time::Instant::now(),
            pass_names: vec![],
        }
    }
}

pub(crate) struct DummyRendererProfiler {}

impl RendererProfilerTrait for DummyRendererProfiler {}
