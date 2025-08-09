use crate::entities::events::{AudioEventTarget, AudioEventTargetId, AudioEventType};
use crate::entities::{Effect, NodeCell};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, PartialEq)]
pub enum MultiplexerEffectEvent {
    Bypass(bool),
    SetDryWet(usize, f32),
}

/// Multiplexer for 1 effect (with the same type)
pub struct Multiplexer1Effect<T1: Effect> {
    id: AudioEventTargetId,
    bypass: bool,
    effect1: NodeCell<T1>,
    wet1: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 2 effects (with different types)
pub struct Multiplexer2Effect<T1: Effect, T2: Effect> {
    id: AudioEventTargetId,
    bypass: bool,
    effect1: NodeCell<T1>,
    effect2: NodeCell<T2>,
    wet1: f32,
    wet2: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 3 effects (with different types)
pub struct Multiplexer3Effect<T1: Effect, T2: Effect, T3: Effect> {
    id: AudioEventTargetId,
    bypass: bool,
    effect1: NodeCell<T1>,
    effect2: NodeCell<T2>,
    effect3: NodeCell<T3>,
    wet1: f32,
    wet2: f32,
    wet3: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 4 effects (with different types)
pub struct Multiplexer4Effect<T1: Effect, T2: Effect, T3: Effect, T4: Effect> {
    id: AudioEventTargetId,
    bypass: bool,
    effect1: NodeCell<T1>,
    effect2: NodeCell<T2>,
    effect3: NodeCell<T3>,
    effect4: NodeCell<T4>,
    wet1: f32,
    wet2: f32,
    wet3: f32,
    wet4: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for N effects, where N is a compile-time constant
/// Note that all effects must have the same type `T`
pub struct MultiplexerEffect<T: Effect, const N: usize> {
    bypass: bool,
    id: AudioEventTargetId,
    effects: [NodeCell<T>; N],
    wet: [f32; N],
    output: PlanarBlock<f32>,
}

fn dispatch_multiplexer<T: Effect, const N: usize>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut MultiplexerEffect<T, N> =
        unsafe { &mut *(ptr as *mut MultiplexerEffect<T, N>) };
    multiplexer.dispatch(event);
}

impl<T: Effect, const N: usize> MultiplexerEffect<T, N> {
    pub fn new(effects: [T; N]) -> Self {
        let effects_refs: [NodeCell<T>; N] = effects.map(|e| NodeCell::new(e));
        MultiplexerEffect {
            bypass: false,
            id: AudioEventTargetId::new(),
            effects: effects_refs,
            wet: [1.0; N],
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_multiplexer::<T, N>, self.id, self)
    }
}

impl<T: Effect, const N: usize> Effect for MultiplexerEffect<T, N> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::MuxEffect(MultiplexerEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(index, wet)) => {
                if *index < N {
                    self.wet[*index] = wet.clamp(0.0, 1.0);
                }
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
        info: &crate::entities::BlockInfo,
    ) {
        todo!()
    }
}

fn dispatch_multiplexer1<T1: Effect>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer1Effect<T1> =
        unsafe { &mut *(ptr as *mut Multiplexer1Effect<T1>) };
    multiplexer.dispatch(event);
}

impl<T1: Effect> Multiplexer1Effect<T1> {
    pub fn new(effect1: T1) -> Self {
        Multiplexer1Effect {
            id: AudioEventTargetId::new(),
            bypass: false,
            effect1: NodeCell::new(effect1),
            wet1: 1.0,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_multiplexer1::<T1>, self.id, self)
    }
}

impl<T1: Effect> Effect for Multiplexer1Effect<T1> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::MuxEffect(MultiplexerEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass;
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(0, wet)) => {
                self.wet1 = wet.clamp(0.0, 1.0);
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
        info: &crate::entities::BlockInfo,
    ) {
        todo!()
    }
}

fn dispatch_multiplexer2<T1: Effect, T2: Effect>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer2Effect<T1, T2> =
        unsafe { &mut *(ptr as *mut Multiplexer2Effect<T1, T2>) };
    multiplexer.dispatch(event);
}

impl<T1: Effect, T2: Effect> Multiplexer2Effect<T1, T2> {
    pub fn new(effect1: T1, effect2: T2) -> Self {
        Multiplexer2Effect {
            id: AudioEventTargetId::new(),
            bypass: false,
            effect1: NodeCell::new(effect1),
            effect2: NodeCell::new(effect2),
            wet1: 1.0,
            wet2: 1.0,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_multiplexer2::<T1, T2>, self.id, self)
    }
}

impl<T1: Effect, T2: Effect> Effect for Multiplexer2Effect<T1, T2> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::MuxEffect(MultiplexerEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(0, wet)) => {
                self.wet1 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(1, wet)) => {
                self.wet2 = wet.clamp(0.0, 1.0);
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
        info: &crate::entities::BlockInfo,
    ) {
        todo!()
    }
}

fn dispatch_multiplexer3<T1: Effect, T2: Effect, T3: Effect>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer3Effect<T1, T2, T3> =
        unsafe { &mut *(ptr as *mut Multiplexer3Effect<T1, T2, T3>) };
    multiplexer.dispatch(event);
}

impl<T1: Effect, T2: Effect, T3: Effect> Multiplexer3Effect<T1, T2, T3> {
    pub fn new(effect1: T1, effect2: T2, effect3: T3) -> Self {
        Multiplexer3Effect {
            id: AudioEventTargetId::new(),
            bypass: false,
            effect1: NodeCell::new(effect1),
            effect2: NodeCell::new(effect2),
            effect3: NodeCell::new(effect3),
            wet1: 1.0,
            wet2: 1.0,
            wet3: 1.0,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_multiplexer3::<T1, T2, T3>, self.id, self)
    }
}

impl<T1: Effect, T2: Effect, T3: Effect> Effect for Multiplexer3Effect<T1, T2, T3> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::MuxEffect(MultiplexerEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(0, wet)) => {
                self.wet1 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(1, wet)) => {
                self.wet2 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(2, wet)) => {
                self.wet3 = wet.clamp(0.0, 1.0);
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
        info: &crate::entities::BlockInfo,
    ) {
        todo!()
    }
}

fn dispatch_multiplexer4<T1: Effect, T2: Effect, T3: Effect, T4: Effect>(
    ptr: *mut u8,
    event: &AudioEventType,
) {
    let multiplexer: &mut Multiplexer4Effect<T1, T2, T3, T4> =
        unsafe { &mut *(ptr as *mut Multiplexer4Effect<T1, T2, T3, T4>) };
    multiplexer.dispatch(event);
}

impl<T1: Effect, T2: Effect, T3: Effect, T4: Effect> Multiplexer4Effect<T1, T2, T3, T4> {
    pub fn new(effect1: T1, effect2: T2, effect3: T3, effect4: T4) -> Self {
        Multiplexer4Effect {
            id: AudioEventTargetId::new(),
            bypass: false,
            effect1: NodeCell::new(effect1),
            effect2: NodeCell::new(effect2),
            effect3: NodeCell::new(effect3),
            effect4: NodeCell::new(effect4),
            wet1: 1.0,
            wet2: 1.0,
            wet3: 1.0,
            wet4: 1.0,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> AudioEventTargetId {
        self.id
    }

    fn create_event_target(&self) -> AudioEventTarget {
        AudioEventTarget::new(dispatch_multiplexer4::<T1, T2, T3, T4>, self.id, self)
    }
}

impl<T1: Effect, T2: Effect, T3: Effect, T4: Effect> Effect for Multiplexer4Effect<T1, T2, T3, T4> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        vec![self.create_event_target()]
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
            AudioEventType::MuxEffect(MultiplexerEffectEvent::Bypass(bypass)) => {
                self.bypass = *bypass
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(0, wet)) => {
                self.wet1 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(1, wet)) => {
                self.wet2 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(2, wet)) => {
                self.wet3 = wet.clamp(0.0, 1.0);
            }
            AudioEventType::MuxEffect(MultiplexerEffectEvent::SetDryWet(3, wet)) => {
                self.wet4 = wet.clamp(0.0, 1.0);
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
        info: &crate::entities::BlockInfo,
    ) {
        todo!()
    }
}
