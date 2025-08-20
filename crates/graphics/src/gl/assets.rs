use crate::gl::entities::material::Material;
use crate::gl::entities::mesh::Mesh;
use crate::gl::entities::shader_program::ShaderProgram;
use crate::gl::entities::texture::Texture;
use crate::passes::events::PassEventTrait;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetType;

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
            |message| {
                if let IRAsset::Shader(shader) = message.ir {
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
            |message| {
                if let IRAsset::Texture(texture) = message.ir {
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

pub(crate) struct MeshAssetFactory {
    basic_factory: BasicFactory<Mesh>,
}

impl MeshAssetFactory {
    pub fn new() -> Self {
        MeshAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Mesh);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Mesh(mesh) = message.ir {
                    Mesh::from_ir(mesh)
                } else {
                    Err("Expected mesh metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Mesh
            },
        );
    }
}

pub(crate) struct MaterialAssetFactory {
    basic_factory: BasicFactory<Material>,
}

impl MaterialAssetFactory {
    pub fn new() -> Self {
        MaterialAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Material);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Material(material) = message.ir {
                    Material::from_ir::<E>(material, message.dependencies)
                } else {
                    Err("Expected material metadata".to_string())
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Mesh
            },
        );
    }
}
