// MacOS (OpenGL 4.1) does not support glQuery for timing
// TODO: Is there an alternative way to do GPU timing on macOS?
#[cfg(target_os = "macos")]
mod timer_impl {
    use std::time::Duration;

    pub struct GPUTimer;

    impl GPUTimer {
        pub fn new(_gl: std::sync::Arc<glow::Context>) -> Option<Self> {
            log::warn!("GPU timing is not supported on macOS due to OpenGL limitations.");
            Some(GPUTimer)
        }

        pub fn start(&mut self) {
            // No-op
        }

        pub fn stop(&mut self) {
            // No-op
        }

        pub fn advance_and_get_time(&mut self) -> Option<Duration> {
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod timer_impl {
    const QUERY_DEPTH: usize = 2;

    use glow::{HasContext, Query};
    use log::debug;
    use std::sync::Arc;
    use std::time::Duration;

    pub struct GPUTimer {
        gl: Arc<glow::Context>,
        queries: [Query; QUERY_DEPTH],
        position: usize,
    }

    impl GPUTimer {
        pub fn new(gl: Arc<glow::Context>) -> Option<Self> {
            debug!("Initializing GPU Timer");

            unsafe {
                let mut queries = Vec::with_capacity(QUERY_DEPTH);
                for _ in 0..QUERY_DEPTH {
                    let query = gl.create_query().ok()?;
                    queries.push(query);
                }
                Some(GPUTimer {
                    gl,
                    queries: queries.try_into().unwrap(),
                    position: 0,
                })
            }
        }

        pub fn start(&mut self) {
            unsafe {
                self.gl
                    .begin_query(glow::TIME_ELAPSED, self.queries[self.position]);
            }
        }

        pub fn stop(&mut self) {
            unsafe {
                self.gl.end_query(glow::TIME_ELAPSED);
            }
        }

        /// Advances to the next query and resolves the time elapsed for the previous one.
        pub fn advance_and_get_time(&mut self) -> Option<Duration> {
            unsafe {
                let prev = (self.position + QUERY_DEPTH - 1) % QUERY_DEPTH;
                let q = self.queries[prev];
                let available = self
                    .gl
                    .get_query_parameter_u32(q, glow::QUERY_RESULT_AVAILABLE);

                if available != 0 {
                    let time_ns = self.gl.get_query_parameter_u32(q, glow::QUERY_RESULT);
                    self.position = (self.position + 1) % QUERY_DEPTH;

                    Some(Duration::from_nanos(time_ns as u64))
                } else {
                    self.position = (self.position + 1) % QUERY_DEPTH;
                    None
                }
            }
        }
    }

    impl Drop for GPUTimer {
        fn drop(&mut self) {
            debug!("Destroying GPU Timer");

            unsafe {
                for query in &self.queries {
                    self.gl.delete_query(*query);
                }
            }
        }
    }
}

pub use timer_impl::GPUTimer;
