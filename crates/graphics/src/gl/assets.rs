use dawn_assets::AssetType;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use crate::gl::entities::shader_program::ShaderProgram;
use crate::gl::entities::texture::Texture;
use crate::passes::events::PassEventTrait;

pub(crate) struct ShaderAssetFactory {
    basic_factory: BasicFactory<ShaderProgram>,
}

impl ShaderAssetFactory {
    pub fn new() -> Self {
        ShaderAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Shader);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self) {
        self.basic_factory.process_events(
            |_, ir| {
                if let IRAsset::Shader(shader) = ir {
                    ShaderProgram::from_ir::<E>(shader)
                } else {
                    Err("Expected shader metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of ShaderProgram
            },
        );
    }
}

pub(crate) struct TextureAssetFactory {
    basic_factory: BasicFactory<Texture>,
}

impl TextureAssetFactory {
    pub fn new() -> Self {
        TextureAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Texture);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self) {
        self.basic_factory.process_events(
            |_, ir| {
                if let IRAsset::Texture(texture) = ir {
                    Texture::from_ir::<E>(texture)
                } else {
                    Err("Expected texture metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Texture
            },
        );
    }
}
