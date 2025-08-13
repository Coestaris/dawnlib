use crate::gl::bindings;
use yage2_core::assets::factory::{BasicFactory, FactoryBinding};
use yage2_core::assets::raw::{AssetRaw, ShaderAssetRaw};
use yage2_core::assets::AssetType;

pub struct ShaderAsset {
    // TODO: id, uniform bindings, etc.
}

impl ShaderAsset {
    fn new(raw: &ShaderAssetRaw) -> Result<ShaderAsset, String> {
        // TODO: Cache the compilation result
        // TODO: Try load SPIRV insteaad of compiling from source
        unsafe {
            let program = bindings::CreateProgram();
            if program == 0 {
                return Err("Failed to create shader program".to_string());
            }
            todo!()
        }
    }
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
        assert_eq!(binding.asset_type(), AssetType::Shader);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |header, raw| {
                // Construct source string from the bytes array
                if let AssetRaw::Shader(shader_raw) = raw {
                    ShaderAsset::new(shader_raw)
                } else {
                    Err("Expected shader metadata".to_string())
                }
            },
            |shader| {
                // Free will be handled in the Drop implementation of ShaderAsset
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
        assert_eq!(binding.asset_type(), AssetType::Texture);
        self.basic_factory.bind(binding);
    }

    pub fn process_events(&mut self) {
        self.basic_factory.process_events(
            |header, raw| {
                // Parse the message to create a ShaderAsset
                // For now, we just return a dummy asset
                Ok(TextureAsset {
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
