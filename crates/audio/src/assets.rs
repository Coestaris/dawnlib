use crate::SampleRate;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::audio::IRAudio;
use dawn_assets::ir::notes::IRNotes;
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetCastable, AssetType};
use dawn_ecs::Tick;
use evenio::component::Component;
use evenio::event::Receiver;
use evenio::fetch::Single;
use evenio::world::World;

#[derive(Debug)]
pub struct AudioAsset(pub IRAudio);

impl AssetCastable for AudioAsset {}

#[derive(Component)]
pub struct AudioAssetFactory {
    sample_rate: SampleRate,
    basic_factory: BasicFactory<AudioAsset>,
}

impl AudioAssetFactory {
    pub fn new(sample_rate: SampleRate) -> Self {
        AudioAssetFactory {
            sample_rate,
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Audio);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |_, ir| {
                if let IRAsset::Audio(data) = ir {
                    // TODO: Resample the audio data to the desired sample rate
                    // For now, we just return the ir data as is.
                    Ok(AudioAsset(data.clone()))
                } else {
                    Err("Expected audio metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of AudioAsset
            },
        );
    }

    pub fn attach_to_ecs(&mut self, world: &mut World) {
        fn handler(_: Receiver<Tick>, mut factory: Single<&mut AudioAssetFactory>) {
            factory.process_events();
        }

        world.add_handler(handler);
    }
}

pub struct NotesAsset(pub IRNotes);

impl AssetCastable for NotesAsset {}

#[derive(Component)]
pub struct NotesAssetFactory {
    basic_factory: BasicFactory<NotesAsset>,
}

impl NotesAssetFactory {
    pub fn new() -> Self {
        NotesAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Notes);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |_, ir| {
                if let IRAsset::Notes(data) = ir {
                    Ok(NotesAsset(data.clone()))
                } else {
                    Err("Expected shader metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of ShaderProgram
            },
        );
    }

    pub fn attach_to_ecs(&mut self, world: &mut World) {
        fn handler(_: Receiver<Tick>, mut factory: Single<&mut NotesAssetFactory>) {
            factory.process_events();
        }

        world.add_handler(handler);
    }
}
