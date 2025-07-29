pub mod bypass;
pub mod fir;
pub mod freeverb;
pub mod multiplexer;
pub mod soft_clip;

#[cfg(test)]
mod test {
    use crate::entities::events::{Event, EventTarget, EventTargetId};
    use crate::entities::{BlockInfo, Effect};
    use crate::sample::PlanarBlock;
    use crate::{BLOCK_SIZE, CHANNELS_COUNT};

    #[derive(Debug, Clone, PartialEq)]
    pub enum TestEffectFunction {
        Add(f32),
        Multiply(f32),
        Constant(f32),
        Clamp { min: f32, max: f32 },
        Square,
    }

    impl TestEffectFunction {
        fn execute(&self, input: f32) -> f32 {
            match self {
                TestEffectFunction::Add(value) => input + value,
                TestEffectFunction::Multiply(value) => input * value,
                TestEffectFunction::Constant(value) => *value,
                TestEffectFunction::Clamp { min, max } => input.clamp(*min, *max),
                TestEffectFunction::Square => input * input,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum TestEffectEvent {
        Bypass(bool),
        SetFunction(TestEffectFunction),
    }

    pub struct TestEffect {
        id: EventTargetId,
        bypass: bool,
        function: TestEffectFunction,
    }

    fn dispatch_test_effect(ptr: *mut u8, event: &Event) {
        let test_effect: &mut TestEffect = unsafe { &mut *(ptr as *mut TestEffect) };
        test_effect.dispatch(event);
    }

    impl TestEffect {
        pub fn new(function: TestEffectFunction) -> Self {
            Self {
                id: EventTargetId::new(),
                bypass: false,
                function,
            }
        }

        pub fn get_id(&self) -> EventTargetId {
            self.id
        }

        fn create_event_target(&self) -> EventTarget {
            EventTarget::new(dispatch_test_effect, self.id, self)
        }
    }

    impl Effect for TestEffect {
        fn get_targets(&self) -> Vec<EventTarget> {
            vec![self.create_event_target()]
        }

        fn dispatch(&mut self, event: &Event) {
            match event {
                Event::TestEffect(TestEffectEvent::Bypass(bypass)) => {
                    self.bypass = *bypass;
                }
                Event::TestEffect(TestEffectEvent::SetFunction(function)) => {
                    self.function = function.clone();
                }
                _ => {}
            }
        }

        fn bypass(&self) -> bool {
            self.bypass
        }

        fn render(
            &mut self,
            input: &PlanarBlock<f32>,
            output: &mut PlanarBlock<f32>,
            info: &BlockInfo,
        ) {
            for channel in 0..CHANNELS_COUNT {
                for index in 0..BLOCK_SIZE {
                    output.samples[channel][index] =
                        self.function.execute(input.samples[channel][index]);
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) use test::*;
