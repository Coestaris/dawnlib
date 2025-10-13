use crate::gl::font::Font;
use crate::gl::material::Material;
use crate::gl::mesh::Mesh;
use crate::gl::raii::shader_program::Program;
use crate::gl::raii::texture::Texture2D;
use crate::passes::events::PassEventTrait;
use dawn_assets::factory::{BasicFactory, FactoryBinding};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetType;
use std::collections::HashMap;
use std::sync::Arc;
use web_time::Duration;

pub(crate) struct ShaderAssetFactory {
    basic_factory: BasicFactory<Program>,
    shader_defines: Arc<dyn Fn() -> HashMap<String, String>>,
}

impl ShaderAssetFactory {
    pub fn new(shader_defines: Arc<dyn Fn() -> HashMap<String, String>>) -> Self {
        ShaderAssetFactory {
            basic_factory: BasicFactory::new(),
            shader_defines,
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Shader);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &Arc<glow::Context>) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Shader(shader) = message.ir {
                    let res = Program::from_ir::<E>(gl.clone(), shader, &(self.shader_defines)())?;
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
    basic_factory: BasicFactory<Texture2D>,
}

impl TextureAssetFactory {
    pub fn new() -> Self {
        TextureAssetFactory {
            basic_factory: BasicFactory::new(),
        }
    }

    pub fn bind(&mut self, binding: FactoryBinding) {
        assert_eq!(binding.asset_type(), AssetType::Texture2D);
        self.basic_factory.bind(binding);
    }

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &Arc<glow::Context>) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Texture(texture) = message.ir {
                    let res = Texture2D::from_ir::<E>(gl.clone(), texture)?;
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &Arc<glow::Context>) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Mesh(mesh) = message.ir {
                    let res = Mesh::from_ir(gl.clone(), mesh, message.dependencies)?;
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

    pub fn process_events<E: PassEventTrait>(&mut self) {
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

    pub fn process_events<E: PassEventTrait>(&mut self, gl: &Arc<glow::Context>) {
        self.basic_factory.process_events(
            |message| {
                if let IRAsset::Font(font) = message.ir {
                    let res = Font::from_ir::<E>(gl.clone(), font, message.dependencies)?;
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
