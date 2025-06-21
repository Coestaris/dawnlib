use ash::{vk, Device, Instance};
use log::debug;
use crate::engine::vulkan::{VkObject, VulkanGraphicsError};

pub(crate) struct Semaphore {
    vk_semaphore: vk::Semaphore,
    name: Option<String>,
}

impl Semaphore {
    pub fn new(device: &Device, name: Option<String>) -> Result<Self, VulkanGraphicsError> {
        debug!("Creating semaphore: {:?}", name);
        let semaphore_create_info = vk::SemaphoreCreateInfo::default();
        let vk_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_create_info, None)
                .map_err(VulkanGraphicsError::SemaphoreCreateFailed)?
        };
        Ok(Semaphore { vk_semaphore, name })
    }

    #[inline]
    pub fn handle(&self) -> vk::Semaphore {
        self.vk_semaphore
    }
}

impl VkObject for Semaphore {
    fn name(&self) -> String {
        self.name.clone().unwrap_or_else(|| "unnamed".to_string())
    }

    fn destroy(&self, _: &Instance, device: &Device) -> Result<(), VulkanGraphicsError> {
        debug!(
            "Destroying semaphore: {} ({:?})",
            self.name(),
            self.vk_semaphore
        );
        unsafe {
            device.destroy_semaphore(self.vk_semaphore, None);
        }

        Ok(())
    }
}

pub(crate) struct Fence {
    pub(crate) vk_fence: vk::Fence,
    name: Option<String>,
}

impl Fence {
    pub fn new(device: &Device, name: Option<String>) -> Result<Self, VulkanGraphicsError> {
        debug!("Creating fence: {:?}", name);
        let fence_create_info = vk::FenceCreateInfo::default();
        let vk_fence = unsafe {
            device
                .create_fence(&fence_create_info, None)
                .map_err(VulkanGraphicsError::FenceCreateFailed)?
        };
        Ok(Fence { vk_fence, name })
    }

    pub fn reset(&self, device: &Device) -> Result<(), VulkanGraphicsError> {
        unsafe {
            device
                .reset_fences(&[self.vk_fence])
                .map_err(VulkanGraphicsError::FenceResetFailed)?;
        }
        Ok(())
    }

    pub fn wait(&self, device: &Device, timeout: u64) -> Result<(), VulkanGraphicsError> {
        unsafe {
            device
                .wait_for_fences(&[self.vk_fence], true, timeout)
                .map_err(VulkanGraphicsError::FenceWaitFailed)?;
        }
        Ok(())
    }
}

impl VkObject for Fence {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_fence".to_string())
    }

    fn destroy(&self, _: &Instance, device: &Device) -> Result<(), VulkanGraphicsError> {
        debug!("Destroying fence: {} ({:?})", self.name(), self.vk_fence);
        unsafe {
            device.destroy_fence(self.vk_fence, None);
        }

        Ok(())
    }
}
