use crate::entities::events::AudioEvent;
use crate::player::Player;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::handler::IntoHandler;
use evenio::world::World;

fn audio_events_handler(r: Receiver<AudioEvent>, player: Single<&Player>) {
    // Remap the event to the player (usually run in the different thread)
    player.0.push_event(r.event);
}

pub fn setup_audio_ecs(world: &mut World, player: Player) {
    // Setup the audio player
    let player_entity = world.spawn();
    world.insert(player_entity, player);

    // Setup the audio events handler
    world.add_handler(audio_events_handler.low());
}
