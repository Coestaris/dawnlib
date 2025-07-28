use common::logging::CommonLogger;
use evenio::prelude::*; // Import the most commonly used items.
use glam::*;

#[derive(Component, Debug)]
struct Position(Vec3);
#[derive(Component, Debug)]
struct PlayingSample {
    id: usize,
    gain: f32,
}
#[derive(GlobalEvent)]
struct Tick;

// Viewpoint or listener head.
#[derive(Component, Debug)]
struct Head {
    direction: Vec3,
    position: Vec3,
}

#[derive(Component, Debug)]
struct AudioEcsAdapter {
    
}

fn audio_ecs_adapter(
    r: Receiver<Tick>,
    head: Single<&Head>,
    private_data: Single<&mut AudioEcsAdapter>,
    positions: Fetcher<(EntityId, &Position, &PlayingSample)>,
) {
    log::debug!("Head: {:?}", head.0);
    for (me, pos, sample) in positions.iter() {
        log::debug!("Me - {:?}, Position - {:?}, Sample - {:?}", me, pos, sample);
    }
}

fn main() {
    // Initialize the logger
    log::set_logger(&CommonLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let mut world = World::new(); // Create an empty world.

    let entity1 = world.spawn();
    world.insert(entity1, Position(Vec3::new(1.0, 2.0, 3.0)));
    let entity2 = world.spawn();
    world.insert(entity2, Position(Vec3::new(4.0, 5.0, 6.0)));
    let entity3 = world.spawn();
    world.insert(entity3, Position(Vec3::new(7.0, 8.0, 9.0)));

    let head = world.spawn();
    world.insert(
        head,
        Head {
            direction: Vec3::new(0.0, 0.0, -1.0),
            position: Vec3::new(0.0, 0.0, 0.0),
        },
    );

    world.add_handler(audio_ecs_adapter.low());
    let adapter = world.spawn();
    world.insert(adapter, AudioEcsAdapter {});

    // loop {
    world.send(Tick);
    world.insert(entity2, PlayingSample { id: 1, gain: 0.5 });
    world.send(Tick);
    // }
}
