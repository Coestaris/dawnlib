use crate::sample::PlanarBlock;
use std::cell::{RefCell, UnsafeCell};
use std::rc::Rc;
use std::borrow::BorrowMut;

const MAX_ACTORS: usize = 1024;

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

trait Effect {
    fn bypass(&self) -> bool;
    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>);
}

trait Source {
    fn frame_start(&mut self) {
        // Default implementation does nothing
    }
    fn render(&mut self) -> &PlanarBlock<f32>;
}

struct Mix1Source<A> {
    a: Rc<A>,
    output: PlanarBlock<f32>,
}

impl<A> Source for Mix1Source<A>
where
    A: Source,
{
    #[inline(always)]
    fn frame_start(&mut self) {
        self.a.frame_start();
    }

    #[inline(always)]
    fn render(&mut self) -> &PlanarBlock<f32> {
        // TODO: Caching logic
        let mut a = self.a.borrow_mut();
        let input = a.render();
        self.output.copy_from(input);
        &self.output
    }
}

struct Mix2Source<A, B>
where
    A: Source,
    B: Source,
{
    a: Rc<A>,
    b: Rc<B>,
    output: PlanarBlock<f32>,
}

impl<A, B> Source for Mix2Source<A, B>
where
    A: Source,
    B: Source,
{
    #[inline(always)]
    fn frame_start(&mut self) {
        self.a.borrow_mut().frame_start();
        self.b.borrow_mut().frame_start();
    }

    #[inline(always)]
    fn render(&mut self) -> &PlanarBlock<f32> {
        let mut a = self.a.borrow_mut();
        let mut b = self.b.borrow_mut();
        let input_a = a.render();
        let input_b = b.render();
        self.output.silence();
        self.output.mix(input_a);
        self.output.mix(input_b);
        &self.output
    }
}

struct Bus<E, S>
where
    E: Effect,
    S: Source,
{
    gain: f32,
    effect: E,
    source: RefCell<S>,

    output: PlanarBlock<f32>,
}

impl<E, S> Bus<E, S>
where
    E: Effect,
    S: Source,
{
    pub fn new(gain: f32, effect: E, source: RefCell<S>) -> Self {
        Self {
            gain: gain.clamp(0.0, 1.0),
            effect,
            source,
            output: PlanarBlock::default(),
        }
    }
}
impl<E, S> Source for Bus<E, S>
where
    E: Effect,
    S: Source,
{
    fn frame_start(&mut self) {
        self.source.borrow_mut().frame_start();
    }

    fn render(&mut self) -> &PlanarBlock<f32> {
        // Render the source
        let mut source = self.source.borrow_mut();
        let input = source.render();

        // Apply the effect
        if !self.effect.bypass() {
            self.effect.process(input, &mut self.output);
        } else {
            self.output.copy_from(input);
        }

        // Apply gain
        for channel in 0..self.output.samples.len() {
            for sample in &mut self.output.samples[channel] {
                *sample *= self.gain;
            }
        }

        &self.output
    }
}

struct BypassEffect {}
impl Effect for BypassEffect {
    fn bypass(&self) -> bool {
        return true;
    }

    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        unreachable!()
    }
}

struct ActorCollection {
    positions: [Vec3; MAX_ACTORS],
    gains: [f32; MAX_ACTORS],
    sends: [f32; MAX_ACTORS],
}

struct SoftClip {}
impl Effect for SoftClip {
    fn bypass(&self) -> bool {
        todo!()
    }

    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        todo!()
    }
}

struct ClipsSource {}
impl Source for ClipsSource {
    fn render(&mut self) -> &PlanarBlock<f32> {
        todo!()
    }
}

struct FreeVerb {}
impl Effect for FreeVerb {
    fn bypass(&self) -> bool {
        todo!()
    }

    fn process(&self, input: &PlanarBlock<f32>, output: &mut PlanarBlock<f32>) {
        todo!()
    }
}

fn d() {
    let actors_effect = BypassEffect {};
    let clip_source = ClipsSource {};
    let actors_bus = Bus::new(1.0, actors_effect, RefCell::new(clip_source));

    let freeverb = FreeVerb {};
    let send_bus = Bus::new(
        1.0,
        freeverb,
        RefCell::new(Mix1Source {
            a: actors_bus,
            output: PlanarBlock::default(),
        }),
    );

    let soft_clip = SoftClip {};
    let master_bus = Bus::new(
        1.0,
        soft_clip,
        RefCell::new(
            Mix2Source {
                a: send_bus,
                b: actors_bus,
                output: PlanarBlock::default(),
            },
        ),
    );
}
