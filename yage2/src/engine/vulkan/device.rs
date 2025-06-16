use crate::engine::vulkan::VulkanGraphicsError;
use ash::{vk, Instance};
use log::{debug, info, trace};
use std::ffi::CStr;

pub unsafe fn get_physical_device(
    instance: &Instance,
) -> Result<vk::PhysicalDevice, VulkanGraphicsError> {
    let devices = instance
        .enumerate_physical_devices()
        .map_err(VulkanGraphicsError::EnumeratePhysicalDevicesError)?;

    if devices.is_empty() {
        return Err(VulkanGraphicsError::EnumeratePhysicalDevicesError(
            vk::Result::ERROR_INITIALIZATION_FAILED,
        ));
    }

    /* Log the available physical devices */
    trace!("Available Vulkan physical devices:");
    for device in &devices {
        let props = instance.get_physical_device_properties(*device);
        trace!(
            " - {} ({:?}) [api={}.{}.{} ({}), driver={}], vid={}, did={}, type={}",
            CStr::from_ptr(props.device_name.as_ptr()).to_string_lossy(),
            device,
            vk::api_version_major(props.api_version),
            vk::api_version_minor(props.api_version),
            vk::api_version_patch(props.api_version),
            props.api_version,
            props.driver_version,
            props.vendor_id,
            props.device_id,
            match props.device_type {
                vk::PhysicalDeviceType::OTHER => "OTHER",
                vk::PhysicalDeviceType::INTEGRATED_GPU => "INTEGRATED_GPU",
                vk::PhysicalDeviceType::DISCRETE_GPU => "DISCRETE_GPU",
                vk::PhysicalDeviceType::VIRTUAL_GPU => "VIRTUAL_GPU",
                vk::PhysicalDeviceType::CPU => "CPU",
                _ => "Unknown",
            }
        );
    }

    /* TODO: Implement device selection logic */
    let device = devices[0];
    debug!("Using physical device: {:?}", device);

    Ok(device)
}
