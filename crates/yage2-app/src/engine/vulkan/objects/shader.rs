use crate::engine::vulkan::{VkObject, VulkanGraphicsError};
use ash::{vk, Device, Instance};
use log::debug;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
    Geometry,
}

pub(crate) struct Shader {
    vk_shader_module: vk::ShaderModule,
    shader_type: ShaderType,
    name: Option<String>,
}

impl Shader {
    pub fn new_from_file(
        shader_type: ShaderType,
        device: &Device,
        file_path: &str,
        name: Option<String>,
    ) -> Result<Self, VulkanGraphicsError> {
        debug!("Creating shader: {:?} {:?}", file_path, name);

        let mut file = std::fs::File::open(file_path)
            .map_err(|e| VulkanGraphicsError::ShaderFileReadError(e.to_string()))?;
        let data = ash::util::read_spv(&mut file)
            .map_err(|e| VulkanGraphicsError::ShaderValidationError(e.to_string()))?;

        let shader_module_create_info = vk::ShaderModuleCreateInfo::default()
            .code(&data)
            .flags(vk::ShaderModuleCreateFlags::empty());
        
        let vk_shader_module = unsafe {
            device
                .create_shader_module(&shader_module_create_info, None)
                .map_err(VulkanGraphicsError::ShaderModuleCreateError)?
        };

        Ok(Shader {
            vk_shader_module,
            shader_type,
            name,
        })
    }

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.vk_shader_module
    }
}

impl VkObject for Shader {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_shader".to_string())
    }

    fn destroy(&self, _: &Instance, device: &Device) -> Result<(), VulkanGraphicsError> {
        debug!(
            "Destroying shader: {} ({:?})",
            self.name(),
            self.vk_shader_module
        );
        unsafe {
            device.destroy_shader_module(self.vk_shader_module, None);
        }

        Ok(())
    }
}
