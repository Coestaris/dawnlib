use crate::engine::vulkan::objects::surface::Surface;
use crate::engine::vulkan::VulkanGraphicsError;
use crate::engine::vulkan::VulkanGraphicsError::GraphicsQueueFamilyNotFound;
use ash::{vk, Instance};
use log::{debug, trace};

fn enumerate_families(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<Vec<vk::QueueFamilyProperties>, VulkanGraphicsError> {
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

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

    Ok(queue_families)
}

pub fn get_graphics_queue_family_index(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<usize, VulkanGraphicsError> {
    let queue_families = enumerate_families(instance, physical_device)?;

    for (family_index, family) in queue_families.iter().enumerate() {
        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            debug!("Found graphics queue family at index: {}", family_index);
            return Ok(family_index);
        }
    }

    Err(GraphicsQueueFamilyNotFound())
}

pub fn get_presentation_queue_family_index(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    surface: &Surface,
) -> Result<usize, VulkanGraphicsError> {
    let queue_families = enumerate_families(instance, physical_device)?;

    for (family_index, family) in queue_families.iter().enumerate() {
        match surface.supports_queue_family(physical_device, family_index as u32) {
            Ok(true) => {
                debug!("Found presentation queue family at index: {}", family_index);
                return Ok(family_index);
            }
            Ok(false) => continue,
            Err(e) => return Err(e),
        }
    }

    Err(VulkanGraphicsError::PresentationQueueFamilyNotFound())
}
