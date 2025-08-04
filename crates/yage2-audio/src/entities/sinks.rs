use crate::entities::events::{AudioEvent, AudioEventTarget, AudioEventTargetId};
use crate::entities::{BlockInfo, Source};
use crate::sample::{InterleavedBlock, InterleavedSample, MappedInterleavedBuffer};
use crate::{SampleRate, BLOCK_SIZE};
use ringbuf::traits::{Consumer, Observer, Producer, SplitRef};

const ROUTER_CAPACITY: usize = 64;
const RING_BUFFER_CAPACITY: usize = 2048;

/// Wraps a master source and allows interleaved
/// buffered rendering and event dispatching.
pub struct InterleavedSink<T: Source> {
    source: T,
    event_router: [AudioEventTarget; ROUTER_CAPACITY],
    sample_rate: SampleRate,
    ring_buf: ringbuf::StaticRb<InterleavedSample<f32>, RING_BUFFER_CAPACITY>,
    processed: usize,
}

unsafe impl<T: Source> Send for InterleavedSink<T> {}
unsafe impl<T: Source> Sync for InterleavedSink<T> {}

impl<T: Source> InterleavedSink<T> {
    // This takes addresses of the master's components.
    // If they are not statically allocated on the heap, UB may occur.
    // Use with caution. 
    pub fn new(master: T, sample_rate: SampleRate) -> Self {
        let targets = master.get_targets();
        let mut event_router: [AudioEventTarget; ROUTER_CAPACITY] =
            [AudioEventTarget::default(); ROUTER_CAPACITY];
        for target in targets {
            event_router[target.get_id().as_usize()] = target;
        }

        InterleavedSink {
            source: master,
            event_router,
            sample_rate,
            ring_buf: ringbuf::StaticRb::default(),
            processed: 0,
        }
    }

    pub(crate) fn dispatch(&self, b: &AudioEvent) {
        let index = b.get_target_id().as_usize();

        #[cfg(debug_assertions)]
        if index >= ROUTER_CAPACITY {
            panic!("InterleavedSink: Event target ID exceeds router capacity");
        }
        #[cfg(debug_assertions)]
        if self.event_router[index].get_id().as_usize() == 0 {
            panic!("InterleavedSink: Event target ID {} is not registered in the router", index);
        }

        // Dispatch the event to the target
        self.event_router[index].dispatch(b.get_event());
    }

    /// Renders a specific number of samples into the output buffer.
    /// Output is allowed to have any number of samples (but less then BLOCK_SIZE)
    /// Render works as a kind of ring-buffer, so we can render only by BLOCK_SIZE samples
    pub(crate) fn render(&mut self, output: &mut MappedInterleavedBuffer<f32>) {
        if output.len > RING_BUFFER_CAPACITY {
            panic!(
                "InterleavedSink: Output buffer length exceeds ({} > {})",
                output.len, RING_BUFFER_CAPACITY
            );
        }

        // If theres not enough samples in the ring buffer, we need to fill it
        while self.ring_buf.split_ref().0.occupied_len() < output.len {
            let info = BlockInfo::new(self.processed, self.sample_rate);
            self.source.frame_start();
            let rendered = self.source.render(&info);
            self.processed += BLOCK_SIZE;

            let mut interleaved_block = InterleavedBlock::default();
            rendered.copy_into_interleaved(&mut interleaved_block);

            // Put the rendered block into the ring buffer
            self.ring_buf.push_slice(&interleaved_block.samples);
        }

        // Now we can read the samples from the ring buffer
        self.ring_buf.pop_slice(output.samples);
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use super::*;
    use crate::dsp::detect_features;
    use crate::entities::bus::Bus;
    use crate::entities::effects::bypass::BypassEffect;
    use crate::entities::sources::TestSource;

    #[test]
    fn sink_test() {
        detect_features();

        let source = TestSource::new();
        let effect = BypassEffect::new();
        let bus = Bus::new(effect, source, None, None);
        let mut sink = InterleavedSink::new(bus, 44100);

        let mut output: [f32; 33 * 2] = [0.0; 33 * 2];
        let mut mapped_output = MappedInterleavedBuffer::new(&mut output).unwrap();

        let mut processed = 0;
        for i in 0..100 {
            sink.render(&mut mapped_output);
            // First 33 samples should be 1.0, 2.0, ..., 33.0
            // Data starts from 1 every 255 samples

            for i in 0..33 {
                let l = mapped_output.samples[i].channels[0];
                let r = mapped_output.samples[i].channels[1];

                let expected = ((processed + i) % BLOCK_SIZE) as f32 + 1.0;

                assert_eq!(l, expected, "Left channel mismatch at sample {}", i);
                assert_eq!(r, expected, "Right channel mismatch at sample {}", i);
            }

            processed += 33;
        }
    }

    #[bench]
    fn bench_sink(b: &mut test::Bencher) {
        detect_features();

        let source = TestSource::new();
        let effect = BypassEffect::new();
        let bus = Bus::new(effect, source, None, None);
        let mut sink = InterleavedSink::new(bus, 44100);

        let mut output: [f32; 33 * 2] = [0.0; 33 * 2];
        let mut mapped_output = MappedInterleavedBuffer::new(&mut output).unwrap();

        b.iter(|| {
            sink.render(&mut mapped_output);
        });
    }
}
