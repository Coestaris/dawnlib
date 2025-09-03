use crate::passes::result::RenderResult;
use crate::passes::MAX_RENDER_PASSES;
use crossbeam_channel::Sender;
use dawn_util::profile::{Counter, MonitorSample, Stopwatch};
use evenio::event::GlobalEvent;
use log::debug;
use std::collections::HashMap;
use std::panic::UnwindSafe;
use std::time::Duration;

#[derive(GlobalEvent, Clone, Debug)]
pub struct RendererMonitorEvent {
    /// Actual number of frames drawn per second.
    pub fps: MonitorSample<f32>,

    /// The time spend on the processing of the view tick
    /// (OS-dependent Window handling).
    pub view: MonitorSample<Duration>,
    /// The time spend on processing the events from the ECS
    /// by the renderer pipeline.
    pub events: MonitorSample<Duration>,

    /// The total time spend on rendering the frame
    /// (including the time spend on the backend).
    pub render: MonitorSample<Duration>,
    /// The amount of time spend on each render pass.
    /// The key is the name of the pass, and the value is the time spent on it.
    /// This is used for profiling the render passes.
    pub passes: Vec<(String, MonitorSample<Duration>)>,

    /// The approximate number of primitives drawn
    /// (triangles, lines, points, etc.) in the frame.
    pub drawn_primitives: MonitorSample<f32>,
    /// The number of draw calls made in the frame.
    /// This is the number of times the GPU was instructed
    pub draw_calls: MonitorSample<f32>,

    /// The total load of the renderer.
    /// Calculated as the ratio of the time spent rendering
    /// to the total frame time (including waiting for VSync).
    pub load: MonitorSample<f32>,
}

pub(crate) trait RendererMonitorTrait: Send + Sync + 'static + UnwindSafe {
    fn set_sender(&mut self, _queue: Sender<RendererMonitorEvent>) {}
    fn set_pass_names(&mut self, _names: &[&str]) {}
    fn view_start(&mut self) {}
    fn view_stop(&mut self) {}

    fn events_start(&mut self) {}
    fn events_stop(&mut self) {}

    fn render_start(&mut self) {}
    fn render_stop(&mut self, _result: RenderResult, _passes: &[Duration; MAX_RENDER_PASSES]) {}
}

pub(crate) struct RendererMonitor {
    fps: Counter,
    view: Stopwatch,
    events: Stopwatch,
    render: Stopwatch,
    draw_calls: Counter,
    drawn_primitives: Counter,
    pass_names: Vec<String>,
    pass_samples: Vec<MonitorSample<Duration>>,
    last_send: std::time::Instant,
    sender: Option<Sender<RendererMonitorEvent>>,
    counter: usize,
}

impl RendererMonitorTrait for RendererMonitor {
    fn set_sender(&mut self, sender: Sender<RendererMonitorEvent>) {
        self.sender = Some(sender);
    }

    fn set_pass_names(&mut self, names: &[&str]) {
        self.pass_names.clear();
        self.pass_samples.clear();
        for name in names {
            self.pass_names.push(name.to_string());
            (self.pass_samples).push(MonitorSample::new(
                Duration::MAX,
                Duration::ZERO,
                Duration::ZERO,
            ));
        }
        debug!("Renderer monitor pass names set: {:?}", self.pass_names);
    }

    fn view_start(&mut self) {
        self.fps.count(1);
        self.view.start();
    }

    fn view_stop(&mut self) {
        self.view.stop();
    }

    fn events_start(&mut self) {
        self.events.start();
    }

    fn events_stop(&mut self) {
        self.events.stop();
    }

    fn render_start(&mut self) {
        self.render.start();
    }

    fn render_stop(&mut self, result: RenderResult, passes: &[Duration; MAX_RENDER_PASSES]) {
        self.render.stop();

        if let RenderResult::Ok { primitives, calls } = result {
            // Update the monitor with the number of primitives and draw calls
            self.drawn_primitives.count(primitives);
            self.draw_calls.count(calls);
        }

        for (i, pass_time) in passes.iter().enumerate() {
            if i < self.pass_samples.len() {
                let sample = &mut self.pass_samples[i];
                let ms = pass_time.as_millis() as f32;
                let average = sample.average().as_millis() as f32;
                let average = average + (ms - average) * 0.5; // Smoothing factor

                // Update the sample with the new time
                self.pass_samples[i] = MonitorSample::new(
                    sample.min().min(*pass_time),
                    sample.max().max(*pass_time),
                    Duration::from_millis(average as u64),
                );
            }
        }

        // Call these each second
        let now = std::time::Instant::now();
        if now.duration_since(self.last_send) >= Duration::from_millis(200) {
            self.last_send = now;
            self.fps.update();
            self.drawn_primitives.update();
            self.draw_calls.update();

            if let Some(sender) = &self.sender {
                let mut passes = Vec::with_capacity(self.pass_names.len());
                for (i, name) in self.pass_names.iter().enumerate() {
                    passes.push((name.clone(), self.pass_samples[i].clone()));
                }

                let fps = self.fps.get().unwrap_or_default();
                let view = self.view.get().unwrap_or_default();
                let events = self.events.get().unwrap_or_default();
                let render = self.render.get().unwrap_or_default();

                let min_time = view.min() + events.min() + render.min();
                let average_time = view.average() + events.average() + render.average();
                let max_time = view.max() + events.max() + render.max();

                let load = MonitorSample::new(
                    min_time.as_secs_f32() * fps.min(),
                    average_time.as_secs_f32() * fps.average(),
                    max_time.as_secs_f32() * fps.max(),
                );

                let frame = RendererMonitorEvent {
                    fps,
                    view,
                    events,
                    render,
                    passes,
                    drawn_primitives: self.drawn_primitives.get().unwrap_or_default(),
                    draw_calls: self.draw_calls.get().unwrap_or_default(),
                    load,
                };

                // Dont care if it fails, the receiver might be gone
                let _ = sender.send(frame);
            }

            // Reset the counters each 5 seconds to get more smooth data
            if self.counter % 50 == 0 {
                self.fps.reset();
                self.view.reset();
                self.events.reset();
                self.render.reset();
                self.draw_calls.reset();
                self.drawn_primitives.reset();
            }
            self.counter += 1;
        }
    }
}
impl RendererMonitor {
    pub fn new() -> Self {
        RendererMonitor {
            fps: Counter::new(0.5),
            view: Stopwatch::new(0.5),
            events: Stopwatch::new(0.5),
            render: Stopwatch::new(0.5),
            draw_calls: Counter::new(0.5),
            drawn_primitives: Counter::new(0.5),
            pass_names: Vec::with_capacity(MAX_RENDER_PASSES),
            pass_samples: Vec::with_capacity(MAX_RENDER_PASSES),
            last_send: std::time::Instant::now(),
            sender: None,
            counter: 0,
        }
    }
}

pub(crate) struct DummyRendererMonitor {}

impl RendererMonitorTrait for DummyRendererMonitor {}
