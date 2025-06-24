use crate::engine::vulkan::{VulkanGraphicsError};
use ash::{vk, Instance};
use log::{debug, trace, warn};
use std::ffi::{c_char, CStr};
use crate::engine::vulkan::graphics::VulkanGraphicsInitArgs;
use crate::engine::vulkan::objects::{get_required_device_extensions, get_wanted_device_extensions};

fn get_supported_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .map_err(VulkanGraphicsError::EnumerateExtensionsError)?
    };

    /* Log the available extensions */
    if !extensions.is_empty() {
        trace!("Available Vulkan device extensions:");
        for ext in &extensions {
            trace!(
                " - {} [ver={}.{}.{} ({})]",
                unsafe { CStr::from_ptr(ext.extension_name.as_ptr()).to_string_lossy() },
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

pub fn get_device_extensions(
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
                unsafe { CStr::from_ptr(wanted).to_string_lossy() }
            );
        }
    }

    if !required.is_empty() {
        debug!("Required Vulkan device extensions:");
        for ext in &required {
            debug!(" - {}", unsafe { CStr::from_ptr(*ext).to_string_lossy() });
        }
    }

    Ok(required)
}

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
