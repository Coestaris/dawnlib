use ash::{vk, Device, Instance};
use ash::vk::{CommandBufferResetFlags, CommandBufferUsageFlags, CommandPoolCreateFlags};
use log::debug;
use crate::engine::vulkan::{VkObject, VulkanGraphicsError};

pub(crate) struct CommandBuffer {
    pub(crate) vk_command_buffer: vk::CommandBuffer,
    // currently supported only for primary command buffers
    pub(crate) vk_command_pool: vk::CommandPool,
    name: Option<String>,
}

impl CommandBuffer {
    pub fn new(
        device: &Device,
        queue_family_index: usize,
        flags: CommandPoolCreateFlags,
        name: Option<String>,
    ) -> Result<Self, VulkanGraphicsError> {
        debug!(
            "Creating command buffer with queue family index: {}, flags: {:?}, name: {:?}",
            queue_family_index, flags, name
        );

        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(flags)
            .queue_family_index(queue_family_index as u32);
        let vk_command_pool = unsafe {
            device
                .create_command_pool(&command_pool_create_info, None)
                .map_err(VulkanGraphicsError::CommandPoolCreateFailed)?
        };
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(vk_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let vk_command_buffers = unsafe {
            device
                .allocate_command_buffers(&allocate_info)
                .map_err(VulkanGraphicsError::CommandBufferAllocateFailed)?
        };
        if vk_command_buffers.is_empty() {
            return Err(VulkanGraphicsError::CommandBufferAllocateFailed(
                vk::Result::ERROR_OUT_OF_DEVICE_MEMORY,
            ));
        }
        let vk_command_buffer = vk_command_buffers[0];

        Ok(CommandBuffer {
            vk_command_buffer,
            vk_command_pool,
            name,
        })
    }

    pub fn draw(
        &self,
        device: &Device,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            device.cmd_draw(
                self.vk_command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn begin(
        &self,
        device: &Device,
        flags: CommandBufferUsageFlags,
    ) -> Result<(), VulkanGraphicsError> {
        let begin_info = vk::CommandBufferBeginInfo::default().flags(flags);
        unsafe {
            device
                .begin_command_buffer(self.vk_command_buffer, &begin_info)
                .map_err(VulkanGraphicsError::CommandBufferBeginFailed)?;
        }
        Ok(())
    }

    pub fn end(&self, device: &Device) -> Result<(), VulkanGraphicsError> {
        unsafe {
            device
                .end_command_buffer(self.vk_command_buffer)
                .map_err(VulkanGraphicsError::CommandBufferEndFailed)?;
        }
        Ok(())
    }

    pub fn reset(
        &self,
        device: &Device,
        flags: CommandBufferResetFlags,
    ) -> Result<(), VulkanGraphicsError> {
        unsafe {
            device
                .reset_command_buffer(self.vk_command_buffer, flags)
                .map_err(VulkanGraphicsError::CommandBufferResetFailed)?;
        }
        Ok(())
    }
}

impl VkObject for CommandBuffer {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_command_buffer".to_string())
    }

    fn destroy(&self, _: &Instance, device: &Device) -> Result<(), VulkanGraphicsError> {
        debug!(
            "Destroying command buffer: {} ({:?})",
            self.name(),
            self.vk_command_buffer
        );
        unsafe {
            device.free_command_buffers(self.vk_command_pool, &[self.vk_command_buffer]);
            device.destroy_command_pool(self.vk_command_pool, None);
        }

        Ok(())
    }
}
