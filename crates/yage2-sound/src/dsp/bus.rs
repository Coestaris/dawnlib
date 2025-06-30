use crate::dsp::{BlockInfo, Control, Generator, Processor, ProcessorType, SourceType};
use crate::sample::PlanarBlock;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum BusMessage {
    SetPan(f32),
    SetVolume(f32),
    SetInvertPhase(bool),
}

pub struct BusControllable {
    pan: f32,    // -1.0 to 1.0
    volume: f32, // 0 to 1.0
    invert_phase: bool,
    processors: Vec<ProcessorType>,
    source: SourceType,
}

pub struct Bus {
    controllable: BusControllable,
    receiver: Receiver<BusMessage>,
}

impl Bus {
    pub fn new() -> BusControllable {
        BusControllable {
            pan: 0.0,                     // Centered
            volume: 1.0,                  // Full volume
            invert_phase: false,          // Normal phase
            processors: Vec::new(),       // No processors by default
            source: SourceType::NoSource, // No source by default
        }
    }
}

impl BusControllable {
    pub fn build(self) -> (Bus, Sender<BusMessage>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let bus = Bus {
            controllable: self,
            receiver,
        };
        (bus, sender)
    }

    pub fn set_pan(mut self, pan: f32) -> Self {
        BusControllable::set_pan_inner(&mut self, pan);
        self
    }

    pub fn set_volume(mut self, volume: f32) -> Self {
        BusControllable::set_volume_inner(&mut self, volume);
        self
    }

    pub fn set_invert_phase(mut self, invert: bool) -> Self {
        BusControllable::set_invert_phase_inner(&mut self, invert);
        self
    }

    pub fn add_processor(mut self, processor: ProcessorType) -> Self {
        self.processors.push(processor);
        self
    }

    pub fn set_source(mut self, source: SourceType) -> Self {
        self.source = source;
        self
    }

    fn set_pan_inner(data: &mut BusControllable, pan: f32) {
        data.pan = pan.clamp(-1.0, 1.0);
    }

    fn set_volume_inner(data: &mut BusControllable, volume: f32) {
        data.volume = volume.clamp(0.0, 1.0);
    }

    fn set_invert_phase_inner(data: &mut BusControllable, invert: bool) {
        data.invert_phase = invert;
    }
}

impl Control for Bus {
    fn process_events(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                BusMessage::SetPan(pan) => {
                    BusControllable::set_pan_inner(&mut self.controllable, pan)
                }
                BusMessage::SetVolume(volume) => {
                    BusControllable::set_volume_inner(&mut self.controllable, volume)
                }
                BusMessage::SetInvertPhase(invert) => {
                    BusControllable::set_invert_phase_inner(&mut self.controllable, invert)
                }
            };
        }

        // Process events for the source
        self.controllable.source.process_events();

        // Process events for each processor
        for processor in &mut self.controllable.processors {
            processor.process_events();
        }
    }
}

impl Generator for Bus {
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        // Generate samples for the source
        self.controllable.source.generate(output, info);

        // Apply processors
        if !self.controllable.processors.is_empty() {
            // Use two blocks to alternate processing
            // This avoids unnecessary allocations and allows for in-place processing
            // block_a is the initial output block
            let mut block_b = PlanarBlock::default();
            let mut use_a = true;

            for processor in &self.controllable.processors {
                if use_a {
                    processor.process(output, &mut block_b, info);
                } else {
                    processor.process(&block_b, output, info);
                }

                // Alternate between block_a and block_b
                use_a = !use_a;
            }

            // Copy the final processed block back to output
            if !use_a {
                output.copy_from(&block_b);
            }
        }

        // Apply pan and volume p
        let panning = self.controllable.pan.clamp(-1.0, 1.0);
        let volume = self.controllable.volume.clamp(0.0, 1.0);
        output.pan_gain_phase_clamp(panning, volume, self.controllable.invert_phase);
    }
}
