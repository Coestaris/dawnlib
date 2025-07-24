use crate::entities::{BlockInfo, Event, EventTarget, EventTargetId, NodeRef, Source};
use crate::sample::PlanarBlock;
use log::debug;

pub enum MultiplexerSourceEvent {
    ChangeMix(usize, f32),
}

/// Multiplexer for 1 source (with the same type)
pub struct Multiplexer1Source<'a, T1: Source> {
    id: EventTargetId,
    cached: bool,
    source1: NodeRef<'a, T1>,
    mix1: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 2 sources (with different types)
pub struct Multiplexer2Source<'a, T1: Source, T2: Source> {
    id: EventTargetId,
    cached: bool,
    source1: NodeRef<'a, T1>,
    source2: NodeRef<'a, T2>,
    mix1: f32,
    mix2: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 3 sources (with different types)
pub struct Multiplexer3Source<'a, T1: Source, T2: Source, T3: Source> {
    id: EventTargetId,
    cached: bool,
    source1: NodeRef<'a, T1>,
    source2: NodeRef<'a, T2>,
    source3: NodeRef<'a, T3>,
    mix1: f32,
    mix2: f32,
    mix3: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 4 sources (with different types)
pub struct Multiplexer4Source<'a, T1: Source, T2: Source, T3: Source, T4: Source> {
    id: EventTargetId,
    cached: bool,
    source1: NodeRef<'a, T1>,
    source2: NodeRef<'a, T2>,
    source3: NodeRef<'a, T3>,
    source4: NodeRef<'a, T4>,
    mix1: f32,
    mix2: f32,
    mix3: f32,
    mix4: f32,
    output: PlanarBlock<f32>,
}

/// Multiplexer for N sources, where N is a compile-time constant
/// Note that all sources must have the same type `T`
pub struct MultiplexerSource<'a, T: Source, const N: usize> {
    id: EventTargetId,
    cached: bool,
    sources: [NodeRef<'a, T>; N],
    mix: [f32; N],
    output: PlanarBlock<f32>,
}

fn dispatch_multiplexer<T: Source, const N: usize>(ptr: *mut u8, event: &Event) {
    let multiplexer: &mut MultiplexerSource<T, N> =
        unsafe { &mut *(ptr as *mut MultiplexerSource<T, N>) };
    multiplexer.dispatch(event);
}

impl<'a, T: Source, const N: usize> MultiplexerSource<'a, T, N> {
    pub fn new(sources: [&'a T; N], mix: [f32; N]) -> Self {
        let sources_refs: [NodeRef<'a, T>; N] = sources.map(|s| NodeRef::new(s));
        MultiplexerSource {
            id: EventTargetId::new(),
            cached: false,
            sources: sources_refs,
            mix,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_multiplexer::<T, N>, self.id, self)
    }
}

impl<'a, T: Source, const N: usize> Source for MultiplexerSource<'a, T, N> {
    fn get_targets(&self) -> Vec<EventTarget> {
        self.sources
            .iter()
            .flat_map(|source| source.as_ref().get_targets())
            .collect()
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(index, mix)) if *index < N => {
                debug!("ChangeMultiplexerSourceMix of muxN {} to {}", self.id, *mix);
                self.mix[*index] = *mix;
            }

            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;

        // TODO: Make sure that loop unrolling is done by the compiler
        for i in 0..N {
            self.sources[i].as_mut().frame_start();
        }
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        self.output.silence();
        // TODO: Make sure that loop unrolling is done by the compiler
        for i in 0..N {
            let input = self.sources[i].as_mut().render(info);
            self.output.addm(input, self.mix[i]);
        }
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer1<T1: Source>(ptr: *mut u8, event: &Event) {
    let multiplexer: &mut Multiplexer1Source<T1> =
        unsafe { &mut *(ptr as *mut Multiplexer1Source<T1>) };
    multiplexer.dispatch(event);
}

impl<'a, T1: Source> Multiplexer1Source<'a, T1> {
    pub fn new(source1: &'a T1, mix1: f32) -> Self {
        Multiplexer1Source {
            id: EventTargetId::new(),
            cached: false,
            source1: NodeRef::new(source1),
            mix1,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_multiplexer1::<T1>, self.id, self)
    }
}

impl<'a, T1: Source> Source for Multiplexer1Source<'a, T1> {
    fn get_targets(&self) -> Vec<EventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(0, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux1:0 {} to {}",
                    self.id, *mix
                );
                self.mix1 = *mix;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;
        self.source1.as_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        let input = self.source1.as_mut().render(info);
        self.output.silence();
        self.output.addm(input, self.mix1);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer2<T1: Source, T2: Source>(ptr: *mut u8, event: &Event) {
    let multiplexer: &mut Multiplexer2Source<T1, T2> =
        unsafe { &mut *(ptr as *mut Multiplexer2Source<T1, T2>) };
    multiplexer.dispatch(event);
}

impl<'a, T1: Source, T2: Source> Multiplexer2Source<'a, T1, T2> {
    pub fn new(source1: &'a T1, source2: &'a T2, mix1: f32, mix2: f32) -> Self {
        Multiplexer2Source {
            id: EventTargetId::new(),
            cached: false,
            source1: NodeRef::new(source1),
            source2: NodeRef::new(source2),
            mix1,
            mix2,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_multiplexer2::<T1, T2>, self.id, self)
    }
}

impl<'a, T1: Source, T2: Source> Source for Multiplexer2Source<'a, T1, T2> {
    fn get_targets(&self) -> Vec<EventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(0, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux2:0 {} to {}",
                    self.id, *mix
                );
                self.mix1 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(1, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux2:1 {} to {}",
                    self.id, *mix
                );
                self.mix2 = *mix;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;
        self.source1.as_mut().frame_start();
        self.source2.as_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        let input1 = self.source1.as_mut().render(info);
        let input2 = self.source2.as_mut().render(info);
        self.output.silence();
        self.output.addm(input1, self.mix1);
        self.output.addm(input2, self.mix2);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer3<T1: Source, T2: Source, T3: Source>(ptr: *mut u8, event: &Event) {
    let multiplexer: &mut Multiplexer3Source<T1, T2, T3> =
        unsafe { &mut *(ptr as *mut Multiplexer3Source<T1, T2, T3>) };
    multiplexer.dispatch(event);
}

impl<'a, T1: Source, T2: Source, T3: Source> Multiplexer3Source<'a, T1, T2, T3> {
    pub fn new(
        source1: &'a T1,
        source2: &'a T2,
        source3: &'a T3,
        mix1: f32,
        mix2: f32,
        mix3: f32,
    ) -> Self {
        Multiplexer3Source {
            id: EventTargetId::new(),
            cached: false,
            source1: NodeRef::new(source1),
            source2: NodeRef::new(source2),
            source3: NodeRef::new(source3),
            mix1,
            mix2,
            mix3,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_multiplexer3::<T1, T2, T3>, self.id, self)
    }
}

impl<'a, T1: Source, T2: Source, T3: Source> Source for Multiplexer3Source<'a, T1, T2, T3> {
    fn get_targets(&self) -> Vec<EventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.extend(self.source3.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(0, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux3:0 {} to {}",
                    self.id, *mix
                );
                self.mix1 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(1, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux3:1 {} to {}",
                    self.id, *mix
                );
                self.mix2 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(2, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux3:2 {} to {}",
                    self.id, *mix
                );
                self.mix3 = *mix;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;
        self.source1.as_mut().frame_start();
        self.source2.as_mut().frame_start();
        self.source3.as_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        let input1 = self.source1.as_mut().render(info);
        let input2 = self.source2.as_mut().render(info);
        let input3 = self.source3.as_mut().render(info);
        self.output.silence();
        self.output.addm(input1, self.mix1);
        self.output.addm(input2, self.mix2);
        self.output.addm(input3, self.mix3);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer4<T1: Source, T2: Source, T3: Source, T4: Source>(
    ptr: *mut u8,
    event: &Event,
) {
    let multiplexer: &mut Multiplexer4Source<T1, T2, T3, T4> =
        unsafe { &mut *(ptr as *mut Multiplexer4Source<T1, T2, T3, T4>) };
    multiplexer.dispatch(event);
}

impl<'a, T1: Source, T2: Source, T3: Source, T4: Source> Multiplexer4Source<'a, T1, T2, T3, T4> {
    pub fn new(
        source1: &'a T1,
        source2: &'a T2,
        source3: &'a T3,
        source4: &'a T4,
        mix1: f32,
        mix2: f32,
        mix3: f32,
        mix4: f32,
    ) -> Self {
        Multiplexer4Source {
            id: EventTargetId::new(),
            cached: false,
            source1: NodeRef::new(source1),
            source2: NodeRef::new(source2),
            source3: NodeRef::new(source3),
            source4: NodeRef::new(source4),
            mix1,
            mix2,
            mix3,
            mix4,
            output: PlanarBlock::default(),
        }
    }

    pub fn get_id(&self) -> EventTargetId {
        self.id
    }

    fn create_event_target(&self) -> EventTarget {
        EventTarget::new(dispatch_multiplexer4::<T1, T2, T3, T4>, self.id, self)
    }
}

impl<'a, T1: Source, T2: Source, T3: Source, T4: Source> Source
    for Multiplexer4Source<'a, T1, T2, T3, T4>
{
    fn get_targets(&self) -> Vec<EventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.extend(self.source3.as_ref().get_targets());
        targets.extend(self.source4.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &Event) {
        match event {
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(0, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux4:0 {} to {}",
                    self.id, *mix
                );
                self.mix1 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(1, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux4:1 {} to {}",
                    self.id, *mix
                );
                self.mix2 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(2, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux4:2 {} to {}",
                    self.id, *mix
                );
                self.mix3 = *mix;
            }
            Event::Multiplexer(MultiplexerSourceEvent::ChangeMix(3, mix)) => {
                debug!(
                    "ChangeMultiplexerSourceMix of mux4:3 {} to {}",
                    self.id, *mix
                );
                self.mix4 = *mix;
            }
            _ => {}
        }
    }

    #[inline(always)]
    fn frame_start(&mut self) {
        self.cached = false;
        self.source1.as_mut().frame_start();
        self.source2.as_mut().frame_start();
        self.source3.as_mut().frame_start();
        self.source4.as_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self, info: &BlockInfo) -> &PlanarBlock<f32> {
        if self.cached {
            return &self.output;
        }

        let input1 = self.source1.as_mut().render(info);
        let input2 = self.source2.as_mut().render(info);
        let input3 = self.source3.as_mut().render(info);
        let input4 = self.source4.as_mut().render(info);
        self.output.silence();
        self.output.addm(input1, self.mix1);
        self.output.addm(input2, self.mix2);
        self.output.addm(input3, self.mix3);
        self.output.addm(input4, self.mix4);
        self.cached = true;
        &self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::detect_features;
    use crate::entities::sources::TestSource;

    #[test]
    fn test_multiplexer1() {
        detect_features();

        let source = TestSource::new();
        let mut mux = Multiplexer1Source::new(&source, 0.5);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 0.5);
            }
        }
    }

    #[test]
    fn test_multiplexer2() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let mut mux = Multiplexer2Source::new(&source1, &source2, 0.1, 0.2);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 0.1 + (i + 1) as f32 * 0.2);
            }
        }
    }

    #[test]
    fn test_multiplexer3() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let source3 = TestSource::new();
        let mut mux = Multiplexer3Source::new(&source1, &source2, &source3, 0.1, 0.2, 0.3);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 0.1 + (i + 1) as f32 * 0.2 + (i + 1) as f32 * 0.3);
            }
        }
    }

    #[test]
    fn test_multiplexer4() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let source3 = TestSource::new();
        let source4 = TestSource::new();
        let mut mux = Multiplexer4Source::new(&source1, &source2, &source3, &source4, 0.1, 0.2, 0.3, 0.4);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 0.1 + (i + 1) as f32 * 0.2 + (i + 1) as f32 * 0.3 + (i + 1) as f32 * 0.4);
            }
        }
    }

    #[test]
    fn test_multiplexer_n() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let source3 = TestSource::new();
        let mut mux = MultiplexerSource::new(
            [&source1, &source2, &source3],
            [0.5, 0.2, 0.3],
        );

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                let expected = (i + 1) as f32 * (0.5 + 0.2 + 0.3);
                assert_eq!(output.samples[channel][i], expected, "Mismatch at sample {}, channel {}", i, channel);
            }
        }
    }
}
