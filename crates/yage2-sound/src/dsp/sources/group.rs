use crate::dsp::{BlockInfo, Control, Generator};
use crate::sample::PlanarBlock;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum GroupMessage {}

/// Allows combining multiple sources into one,
/// for example, mixing multiple audio clips or generators.
pub struct GroupSource {
    pub busses: Vec<crate::dsp::bus::Bus>,
    receiver: Receiver<GroupMessage>,
}

impl GroupSource {
    pub fn new(busses: Vec<crate::dsp::bus::Bus>) -> (Self, Sender<GroupMessage>) {
        let (sender, receiver) = channel();
        let source = Self { busses, receiver };
        (source, sender)
    }
}

impl Control for GroupSource {
    fn process_events(&mut self) {
        while let Ok(_message) = self.receiver.try_recv() {
            // Process messages if needed
            // Currently, no specific messages are defined for GroupSource
        }

        // Process events for all nested busses
        for bus in &mut self.busses {
            bus.process_events();
        }
    }
}

impl Generator for GroupSource {
    fn generate(&self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        for bus in &self.busses {
            let mut bus_output = PlanarBlock::default();
            bus.generate(&mut bus_output, info);
            output.mix(&bus_output);
        }
    }
}
