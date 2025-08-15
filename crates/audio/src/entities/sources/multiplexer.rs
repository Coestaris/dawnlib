use crate::entities::{
    AudioEventTarget, AudioEventTargetId, AudioEventType, BlockInfo, NodeCell, Source,
};
use crate::sample::PlanarBlock;

#[derive(Debug, Clone, PartialEq)]
pub enum MultiplexerSourceEvent {
    SetGain(usize, f32),
}

/// Multiplexer for 1 source (with the same type)
pub struct Multiplexer1Source<T1: Source> {
    id: AudioEventTargetId,
    cached: bool,
    source1: NodeCell<T1>,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 2 sources (with different types)
pub struct Multiplexer2Source<T1: Source, T2: Source> {
    id: AudioEventTargetId,
    cached: bool,
    source1: NodeCell<T1>,
    source2: NodeCell<T2>,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 3 sources (with different types)
pub struct Multiplexer3Source<T1: Source, T2: Source, T3: Source> {
    id: AudioEventTargetId,
    cached: bool,
    source1: NodeCell<T1>,
    source2: NodeCell<T2>,
    source3: NodeCell<T3>,
    output: PlanarBlock<f32>,
}

/// Multiplexer for 4 sources (with different types)
pub struct Multiplexer4Source<T1: Source, T2: Source, T3: Source, T4: Source> {
    id: AudioEventTargetId,
    cached: bool,
    source1: NodeCell<T1>,
    source2: NodeCell<T2>,
    source3: NodeCell<T3>,
    source4: NodeCell<T4>,
    output: PlanarBlock<f32>,
}

/// Multiplexer for N sources, where N is a compile-time constant
/// Note that all sources must have the same type `T`
pub struct MultiplexerSource<T: Source, const N: usize> {
    id: AudioEventTargetId,
    cached: bool,
    sources: [NodeCell<T>; N],
    output: PlanarBlock<f32>,
}

fn dispatch_multiplexer<T: Source, const N: usize>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut MultiplexerSource<T, N> =
        unsafe { &mut *(ptr as *mut MultiplexerSource<T, N>) };
    multiplexer.dispatch(event);
}

impl<T: Source, const N: usize> MultiplexerSource<T, N> {
    pub fn new(sources: [T; N]) -> Self {
        let sources_refs: [NodeCell<T>; N] = sources.map(|s| NodeCell::new(s));
        MultiplexerSource {
            id: AudioEventTargetId::new(),
            cached: false,
            sources: sources_refs,
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

impl<T: Source, const N: usize> Source for MultiplexerSource<T, N> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        self.sources
            .iter()
            .flat_map(|source| source.as_ref().get_targets())
            .chain(std::iter::once(self.create_event_target()))
            .collect()
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
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
            self.output.add(input);
        }
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer1<T1: Source>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer1Source<T1> =
        unsafe { &mut *(ptr as *mut Multiplexer1Source<T1>) };
    multiplexer.dispatch(event);
}

impl<T1: Source> Multiplexer1Source<T1> {
    pub fn new(source1: T1) -> Self {
        Multiplexer1Source {
            id: AudioEventTargetId::new(),
            cached: false,
            source1: NodeCell::new(source1),
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

impl<T1: Source> Source for Multiplexer1Source<T1> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
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
        self.output.add(input);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer2<T1: Source, T2: Source>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer2Source<T1, T2> =
        unsafe { &mut *(ptr as *mut Multiplexer2Source<T1, T2>) };
    multiplexer.dispatch(event);
}

impl<T1: Source, T2: Source> Multiplexer2Source<T1, T2> {
    pub fn new(source1: T1, source2: T2) -> Self {
        Multiplexer2Source {
            id: AudioEventTargetId::new(),
            cached: false,
            source1: NodeCell::new(source1),
            source2: NodeCell::new(source2),
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

impl<T1: Source, T2: Source> Source for Multiplexer2Source<T1, T2> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
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
        self.output.add(input1);
        self.output.add(input2);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer3<T1: Source, T2: Source, T3: Source>(ptr: *mut u8, event: &AudioEventType) {
    let multiplexer: &mut Multiplexer3Source<T1, T2, T3> =
        unsafe { &mut *(ptr as *mut Multiplexer3Source<T1, T2, T3>) };
    multiplexer.dispatch(event);
}

impl<T1: Source, T2: Source, T3: Source> Multiplexer3Source<T1, T2, T3> {
    pub fn new(source1: T1, source2: T2, source3: T3) -> Self {
        Multiplexer3Source {
            id: AudioEventTargetId::new(),
            cached: false,
            source1: NodeCell::new(source1),
            source2: NodeCell::new(source2),
            source3: NodeCell::new(source3),
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

impl<T1: Source, T2: Source, T3: Source> Source for Multiplexer3Source<T1, T2, T3> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.extend(self.source3.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
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
        self.output.add(input1);
        self.output.add(input2);
        self.output.add(input3);
        self.cached = true;
        &self.output
    }
}

fn dispatch_multiplexer4<T1: Source, T2: Source, T3: Source, T4: Source>(
    ptr: *mut u8,
    event: &AudioEventType,
) {
    let multiplexer: &mut Multiplexer4Source<T1, T2, T3, T4> =
        unsafe { &mut *(ptr as *mut Multiplexer4Source<T1, T2, T3, T4>) };
    multiplexer.dispatch(event);
}

impl<T1: Source, T2: Source, T3: Source, T4: Source> Multiplexer4Source<T1, T2, T3, T4> {
    pub fn new(source1: T1, source2: T2, source3: T3, source4: T4) -> Self {
        Multiplexer4Source {
            id: AudioEventTargetId::new(),
            cached: false,
            source1: NodeCell::new(source1),
            source2: NodeCell::new(source2),
            source3: NodeCell::new(source3),
            source4: NodeCell::new(source4),
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

impl<T1: Source, T2: Source, T3: Source, T4: Source> Source for Multiplexer4Source<T1, T2, T3, T4> {
    fn get_targets(&self) -> Vec<AudioEventTarget> {
        let mut targets = self.source1.as_ref().get_targets();
        targets.extend(self.source2.as_ref().get_targets());
        targets.extend(self.source3.as_ref().get_targets());
        targets.extend(self.source4.as_ref().get_targets());
        targets.push(self.create_event_target());
        targets
    }

    fn dispatch(&mut self, event: &AudioEventType) {
        match event {
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
        self.output.add(input1);
        self.output.add(input2);
        self.output.add(input3);
        self.output.add(input4);
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
        let mut mux = Multiplexer1Source::new(source);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32);
            }
        }
    }

    #[test]
    fn test_multiplexer2() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let mut mux = Multiplexer2Source::new(source1, source2);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 2.0);
            }
        }
    }

    #[test]
    fn test_multiplexer3() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let source3 = TestSource::new();
        let mut mux = Multiplexer3Source::new(source1, source2, source3);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 3.0);
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
        let mut mux = Multiplexer4Source::new(source1, source2, source3, source4);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                assert_eq!(output.samples[channel][i], (i + 1) as f32 * 4.0);
            }
        }
    }

    #[test]
    fn test_multiplexer_n() {
        detect_features();

        let source1 = TestSource::new();
        let source2 = TestSource::new();
        let source3 = TestSource::new();
        let mut mux = MultiplexerSource::new([source1, source2, source3]);

        mux.frame_start();
        let info = BlockInfo::new(0, 44_100);
        let output = mux.render(&info);

        // Check that the output is as expected
        for i in 0..output.samples[0].len() {
            for channel in 0..output.samples.len() {
                let expected = (i + 1) as f32 * 3.0;
                assert_eq!(
                    output.samples[channel][i], expected,
                    "Mismatch at sample {}, channel {}",
                    i, channel
                );
            }
        }
    }
}
