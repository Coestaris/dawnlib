use ash::{vk, Device, Instance};
use log::debug;
use crate::engine::vulkan::{VkObject, VulkanGraphicsError};

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
        let code = std::fs::read(file_path)
            .map_err(|_| VulkanGraphicsError::ShaderFileReadError(file_path.to_string()))?;

        Self::new(shader_type, device, &code, name)
    }

    pub fn new(
        shader_type: ShaderType,
        device: &Device,
        code: &[u8],
        name: Option<String>,
    ) -> Result<Self, VulkanGraphicsError> {
        debug!(
            "Creating shader: {} ({:?}) with code size: {} bytes",
            name.as_deref().unwrap_or("unnamed_shader"),
            shader_type,
            code.len()
        );
        let create_info = vk::ShaderModuleCreateInfo {
            p_code: code.as_ptr() as *const u32,
            code_size: code.len(),
            flags: vk::ShaderModuleCreateFlags::empty(),
            ..Default::default()
        };

        let shader_module = unsafe {
            device
                .create_shader_module(&create_info, None)
                .map_err(VulkanGraphicsError::ShaderModuleCreateError)?
        };

        Ok(Shader {
            vk_shader_module: shader_module,
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
