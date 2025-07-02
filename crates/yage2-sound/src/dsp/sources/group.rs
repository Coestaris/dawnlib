use crate::dsp::{BlockInfo, EventDispatcher, Generator};
use crate::control::{new_control, ControlReceiver, Controller};
use crate::sample::PlanarBlock;

pub enum GroupMessage {}

/// Allows combining multiple sources into one,
/// for example, mixing multiple audio clips or generators.
pub struct GroupSource {
    pub busses: Vec<crate::dsp::bus::Bus>,
    receiver: ControlReceiver<GroupMessage>,
}

impl GroupSource {
    pub fn new(busses: Vec<crate::dsp::bus::Bus>) -> (Self, Controller<GroupMessage>) {
        let (controller, receiver) = new_control();
        let source = Self { busses, receiver };
        (source, controller)
    }
}

impl EventDispatcher for GroupSource {
    fn dispatch_events(&mut self) {
        while let Some(_) = self.receiver.receive() {
            // Process messages if needed
            // Currently, no specific messages are defined for GroupSource
        }

        // Process events for all nested busses
        for bus in &mut self.busses {
            bus.dispatch_events();
        }
    }
}

impl Generator for GroupSource {
    fn generate(&mut self, output: &mut PlanarBlock<f32>, info: &BlockInfo) {
        for bus in &mut self.busses {
            let mut bus_output = PlanarBlock::default();
            bus.generate(&mut bus_output, info);
            output.mix(&bus_output);
        }
    }
}
