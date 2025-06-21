use crate::engine::vulkan::{VulkanGraphicsError};
use ash::{vk, Instance};
use log::{debug, trace, warn};
use std::ffi::{c_char, CStr};

pub unsafe fn get_queue_family_index(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<usize, VulkanGraphicsError> {
    let queue_families = instance.get_physical_device_queue_family_properties(physical_device);

    if queue_families.is_empty() {
        return Err(VulkanGraphicsError::EnumerateQueueFamiliesError(
            vk::Result::ERROR_INITIALIZATION_FAILED,
        ));
    }

    /* Log the available queue families */
    trace!("Available Vulkan queue families:");
    for (i, family) in queue_families.iter().enumerate() {
        trace!(
            " - Queue Family {}: {:?} (count={})",
            i,
            family.queue_flags,
            family.queue_count
        );
    }

    /* TODO: Implement queue family selection logic */
    let family_index = 0;
    debug!("Using queue family index: {}", family_index);

    Ok(family_index)
}
