use crate::gl::font::Font;
use crate::gl::material::Material;
use crate::gl::mesh::Mesh;
use crate::gl::raii::shader_program::Program;
use crate::gl::raii::texture::Texture;
use crate::passes::events::PassEventTrait;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetType;
use std::time::Duration;

pub(crate) struct ShaderAssetFactory {
    // Using 'static lifetime here because shader programs are
    // expected to live as long as the application.
    basic_factory: BasicFactory<Program>,
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &'static glow::Context) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Shader(shader) = message.ir {
                    let res = Program::from_ir::<E>(gl, shader)?;
                    Ok(res)
                } else {
                    Err(anyhow::anyhow!("Expected shader metadata"))
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of ShaderProgram
            },
            Duration::ZERO,
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &'static glow::Context) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Texture(texture) = message.ir {
                    let res = Texture::from_ir::<E>(gl, texture)?;
                    Ok(res)
                } else {
                    Err(anyhow::anyhow!("Expected texture metadata"))
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Texture
            },
            Duration::ZERO,
        );
    }
}

pub(crate) struct MeshAssetFactory {
    // About lifetimes see shader program comment
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &'static glow::Context) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Mesh(mesh) = message.ir {
                    let res = Mesh::from_ir(gl, mesh, message.dependencies)?;
                    Ok(res)
                } else {
                    Err(anyhow::anyhow!("Expected mesh metadata"))
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Mesh
            },
            Duration::ZERO,
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &'static glow::Context) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Material(material) = message.ir {
                    let res = Material::from_ir::<E>(material, message.dependencies)?;
                    Ok(res)
                } else {
                    Err(anyhow::anyhow!("Expected material metadata"))
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Mesh
            },
            Duration::ZERO,
        );
    }
}

pub(crate) struct FontAssetFactory {
    // About lifetimes see shader program comment
    basic_factory: BasicFactory<Font>,
}

impl FontAssetFactory {
    pub fn new() -> Self {
        FontAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Font);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &'static glow::Context) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Font(font) = message.ir {
                    let res = Font::from_ir::<E>(gl, font, message.dependencies)?;
                    Ok(res)
                } else {
                    Err(anyhow::anyhow!("Expected font metadata"))
                }
            },
            |_| {
                // Free will be handled in the Drop implementation of Mesh
            },
            Duration::ZERO,
        );
    }
}
