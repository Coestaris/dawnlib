use crate::sample::InterleavedSample;
use crate::{SampleType, RING_BUFFER_SIZE};
use std::cell::UnsafeCell;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Condvar, Mutex,
};

const RING_BUFFER_MASK: usize = RING_BUFFER_SIZE - 1;

pub struct RingBuffer {
    buffer: [UnsafeCell<InterleavedSample<SampleType>>; RING_BUFFER_SIZE],
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    space_cv: (Mutex<()>, Condvar),
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    pub fn new() -> Self {
        RingBuffer {
            buffer: std::array::from_fn(|_| UnsafeCell::new(Default::default())),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            space_cv: (Mutex::new(()), Condvar::new()),
        }
    }

    pub fn write_bulk_wait(&self, min_required: usize) -> RingWriterGuard {
        let (lock, cvar) = &self.space_cv;
        let mut guard = lock.lock().unwrap();

        loop {
            let write = self.write_pos.load(Ordering::Relaxed);
            let read = self.read_pos.load(Ordering::Acquire);
            let available = RING_BUFFER_SIZE - (write - read);

            if available >= min_required {
                break;
            }

            guard = cvar.wait(guard).unwrap();
        }

        RingWriterGuard { ring: self }
    }

    pub fn read_bulk(&self) -> RingReaderGuard {
        RingReaderGuard { ring: self }
    }

    pub fn poison(&self) {
        // Poison the ring buffer by resetting positions
        self.write_pos.store(0, Ordering::Relaxed);
        self.read_pos.store(0, Ordering::Relaxed);

        // Notify any waiting threads that the buffer has been poisoned
        let (_, cvar) = &self.space_cv;
        cvar.notify_all();
    }
}

pub struct RingWriterGuard<'a> {
    ring: &'a RingBuffer,
}

impl<'a> RingWriterGuard<'a> {
    pub fn write_samples(&self, samples: &[InterleavedSample<SampleType>]) -> usize {
        let write = self.ring.write_pos.load(Ordering::Relaxed);
        let read = self.ring.read_pos.load(Ordering::Acquire);
        let available = RING_BUFFER_SIZE - (write - read);
        let count = samples.len().min(available);

        for i in 0..count {
            let index = (write + i) & RING_BUFFER_MASK;
            unsafe {
                *self.ring.buffer[index].get() = samples[i];
            }
        }

        self.ring.write_pos.store(write + count, Ordering::Release);
        count
    }
}

pub struct RingReaderGuard<'a> {
    ring: &'a RingBuffer,
}

impl<'a> RingReaderGuard<'a> {
    pub fn read_or_silence(&self, target: &mut [InterleavedSample<SampleType>]) -> (usize, usize) {
        let write = self.ring.write_pos.load(Ordering::Acquire);
        let read = self.ring.read_pos.load(Ordering::Relaxed);
        let available = write - read;
        let count = target.len();

        let to_read = count.min(available);

        for i in 0..to_read {
            let index = (read + i) & RING_BUFFER_MASK;
            target[i] = unsafe { *self.ring.buffer[index].get() };
        }

        for i in to_read..count {
            target[i] = InterleavedSample::default();
        }

        self.ring.read_pos.store(read + to_read, Ordering::Release);

        self.ring.space_cv.1.notify_one();
        (to_read, available)
    }
}
