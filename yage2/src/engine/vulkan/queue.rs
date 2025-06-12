use crate::engine::vulkan::cfg::{get_required_device_extensions, get_wanted_device_extensions};
use crate::engine::vulkan::{VulkanGraphicsError, VulkanGraphicsInitArgs};
use ash::{vk, Instance};
use log::{debug, info, trace, warn};
use std::ffi::{c_char, CStr};

unsafe fn get_supported_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device)
        .map_err(VulkanGraphicsError::EnumerateExtensionsError)?;

    /* Log the available extensions */
    if !extensions.is_empty() {
        trace!("Available Vulkan device extensions:");
        for ext in &extensions {
            trace!(
                " - {} [ver={}.{}.{} ({})]",
                CStr::from_ptr(ext.extension_name.as_ptr()).to_string_lossy(),
                vk::api_version_major(ext.spec_version),
                vk::api_version_minor(ext.spec_version),
                vk::api_version_patch(ext.spec_version),
                ext.spec_version,
            );
        }
    }

    Ok(extensions
        .iter()
        .map(|ext| ext.extension_name.as_ptr() as *const c_char)
        .collect())
}

pub unsafe fn get_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    init: &VulkanGraphicsInitArgs,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let mut required = get_required_device_extensions()
        .into_iter()
        .chain(init.device_extensions.iter().copied())
        .collect::<Vec<_>>();

    /* If the wanted extension is supported */
    let supported = get_supported_device_extensions(instance, physical_device)?;
    for wanted in get_wanted_device_extensions() {
        if supported.contains(&wanted) {
            required.push(wanted);
        } else {
            warn!(
                "Vulkan device extension {} is not supported, skipping it",
                CStr::from_ptr(wanted).to_string_lossy()
            );
        }
    }

    if !required.is_empty() {
        debug!("Required Vulkan device extensions:");
        for ext in &required {
            debug!(" - {}", CStr::from_ptr(*ext).to_string_lossy());
        }
    }

    Ok(required)
}

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
    info!("Using queue family index: {}", family_index);

    Ok(family_index)
}
