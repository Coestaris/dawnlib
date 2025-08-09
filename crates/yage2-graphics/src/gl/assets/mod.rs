use yage2_core::assets::factory::{BasicFactory, FactoryBinding};
use yage2_core::assets::AssetType;

pub struct ShaderAsset {
    // TODO: id, uniform bindings, etc.
}

pub(crate) struct ShaderAssetFactory {
    basic_factory: BasicFactory<ShaderAsset>,
}

impl ShaderAssetFactory {
    pub fn new() -> Self {
        ShaderAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::ShaderSPIRV);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |header, source| {
                // Parse the message to create a ShaderAsset
                // For now, we just return a dummy asset
                Some(ShaderAsset {
                    // Initialize with data from header and source
                })
            },
            |shader| {
                // Free the asset if needed
                // For now, we do nothing
            },
        );
    }
}

pub struct TextureAsset {
    // TODO: id, uniform bindings, etc.
}

pub(crate) struct TextureAssetFactory {
    basic_factory: BasicFactory<TextureAsset>,
}

impl TextureAssetFactory {
    pub fn new() -> Self {
        TextureAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::ImagePNG);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |header, raw| {
                // Parse the message to create a ShaderAsset
                // For now, we just return a dummy asset
                Some(TextureAsset {
                    // Initialize with data from header and source
                })
            },
            |texture| {
                // Free the asset if needed
                // For now, we do nothing
            },
        );
    }
}
