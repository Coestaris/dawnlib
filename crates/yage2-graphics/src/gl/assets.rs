use crate::gl::entities::{ShaderProgram, Texture};
use crate::passes::events::PassEventTrait;
use crate::renderer::RendererBackend;
use yage2_core::assets::factory::{BasicFactory, FactoryBinding};
use yage2_core::assets::raw::AssetRaw;
use yage2_core::assets::AssetType;

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
            |_, raw| {
                if let AssetRaw::Shader(shader_raw) = raw {
                    ShaderProgram::from_raw::<E>(shader_raw)
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
            |_, raw| {
                if let AssetRaw::Texture(texture_raw) = raw {
                    Texture::from_raw::<E>(texture_raw)
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
