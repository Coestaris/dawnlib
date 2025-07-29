pub mod actor;
pub mod multiplexer;
pub mod waveform;

#[cfg(test)]
mod test {
    use crate::entities::events::{Event, EventTarget, EventTargetId};
    use crate::entities::{BlockInfo, Source};
    use crate::sample::PlanarBlock;
    use crate::BLOCK_SIZE;

    #[derive(Debug, Clone, PartialEq)]
    pub enum TestSourceEvent {
        SetMultiplier(f32),
    }

    pub(crate) struct TestSource {
        id: EventTargetId,
        cached: bool,
        mul: f32,
        output: PlanarBlock<f32>,
    }

    fn dispatch_test_source(ptr: *mut u8, event: &Event) {
        let source: &mut TestSource = unsafe { &mut *(ptr as *mut TestSource) };
        // Here you would typically handle the event, but for this example, we do nothing.
        source.dispatch(event);
    }

    /// Generates a 1,2,3,4,... sequence of numbers as a "signal"
    impl TestSource {
        pub fn new() -> Self {
            Self {
                id: EventTargetId::new(),
                cached: false,
                output: PlanarBlock::default(),
                mul: 1.0, // Default multiplier
            }
        }

        pub fn get_id(&self) -> EventTargetId {
            self.id
        }

        fn create_event_target(&self) -> EventTarget {
            EventTarget::new(dispatch_test_source, self.id, self)
        }

        fn generate_test_signal(output: &mut PlanarBlock<f32>, mul: f32) {
            // Fill the output block with a simple test signal, e.g., a sequence of numbers
            for i in 0..BLOCK_SIZE {
                for channel in 0..output.samples.len() {
                    output.samples[channel][i] = (i + 1) as f32 * mul; // Fill with 1, 2, 3, ...
                }
            }
        }
    }

    impl Source for TestSource {
        fn get_targets(&self) -> Vec<EventTarget> {
            vec![self.create_event_target()]
        }

        fn dispatch(&mut self, event: &Event) {
            match event {
                Event::TestSource(TestSourceEvent::SetMultiplier(mul)) => {
                    self.mul = *mul;
                    // Here you might want to update the output based on the new multiplier
                    // For simplicity, we won't modify the output in this example
                }
                _ => {}
            }
        }

        fn frame_start(&mut self) {
            // Reset the cached state at the start of each frame
            self.cached = false;
        }

        fn render(&mut self, _info: &BlockInfo) -> &PlanarBlock<f32> {
            if self.cached {
                return &self.output;
            }

            self.cached = true;
            TestSource::generate_test_signal(&mut self.output, self.mul);
            &self.output
        }
    }

    #[cfg(test)]
    mod tests {
        extern crate test;
        use super::*;

        #[test]
        fn test_test_source() {
            let mut source = TestSource::new();
            let info = BlockInfo::new(0, 44100); // Sample index and sample rate are arbitrary for this test

            // Render the source
            let output = source.render(&info);

            // Check if the output is filled with the expected values
            for i in 0..BLOCK_SIZE {
                for channel in 0..output.samples.len() {
                    assert_eq!(output.samples[channel][i], (i + 1) as f32);
                }
            }
        }

        #[bench]
        fn bench_test_source(b: &mut test::Bencher) {
            let mut source = TestSource::new();
            let info = BlockInfo::new(0, 44100); // Sample index and sample rate are arbitrary for this test

            b.iter(|| {
                source.frame_start();
                source.render(&info);
            });
        }
    }
}

#[cfg(test)]
pub(crate) use test::*;
