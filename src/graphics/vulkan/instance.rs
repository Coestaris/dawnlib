use crate::graphics::vulkan::cfg::{
    get_required_instance_extensions, get_required_layers, get_wanted_instance_extensions,
    get_wanted_layers,
};
use crate::graphics::vulkan::{VulkanGraphicsError, VulkanGraphicsInitArgs};
use crate::utils::contains;
use ash::vk;
use log::{debug, trace, warn};
use std::ffi::{c_char, CStr};

unsafe fn get_supported_instance_extensions(
    entry: &ash::Entry,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let extensions = entry
        .enumerate_instance_extension_properties(None)
        .map_err(VulkanGraphicsError::EnumerateExtensionsError)?;

    /* Log the available extensions */
    if !extensions.is_empty() {
        trace!("Available Vulkan Instance extensions:");
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

pub unsafe fn get_instance_extensions(
    entry: &ash::Entry,
    init: &VulkanGraphicsInitArgs,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let mut required = get_required_instance_extensions()
        .into_iter()
        .chain(init.instance_extensions.iter().copied())
        .collect::<Vec<_>>();

    /* If the wanted extension is supported */
    let supported = get_supported_instance_extensions(&entry)?;
    for wanted in get_wanted_instance_extensions() {
        if contains(&supported, wanted) {
            required.push(wanted);
        } else {
            warn!(
                "Vulkan instance extension {} is not supported, skipping it",
                CStr::from_ptr(wanted).to_string_lossy()
            );
        }
    }

    /* log the required extensions */
    if !required.is_empty() {
        debug!("Required Vulkan instance extensions:");
        for ext in &required {
            debug!(" - {}", CStr::from_ptr(*ext).to_string_lossy());
        }
    }

    Ok(required)
}

unsafe fn get_supported_layers(
    entry: &ash::Entry,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let layers = entry
        .enumerate_instance_layer_properties()
        .map_err(VulkanGraphicsError::EnumerateLayersError)?;

    /* Log the available layers */
    if !layers.is_empty() {
        trace!("Available Vulkan layers:");
        for layer in &layers {
            trace!(
                " - {} [ver={}.{}.{} ({}), impl={}] - {}",
                CStr::from_ptr(layer.layer_name.as_ptr()).to_string_lossy(),
                vk::api_version_major(layer.spec_version),
                vk::api_version_minor(layer.spec_version),
                vk::api_version_patch(layer.spec_version),
                layer.spec_version,
                layer.implementation_version,
                CStr::from_ptr(layer.description.as_ptr()).to_string_lossy()
            );
        }
    }

    Ok(layers
        .iter()
        .map(|layer| layer.layer_name.as_ptr() as *const c_char)
        .collect())
}

pub unsafe fn get_layers(
    entry: &ash::Entry,
    init: &VulkanGraphicsInitArgs,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let mut required = get_required_layers()
        .into_iter()
        .chain(init.layers.iter().copied())
        .collect::<Vec<_>>();

    /* If the wanted layer is supported */
    let supported = get_supported_layers(&entry)?;
    for wanted in get_wanted_layers() {
        if contains(&supported, wanted) {
            required.push(wanted);
        } else {
            warn!(
                "Vulkan layer {} is not supported, skipping it",
                CStr::from_ptr(wanted).to_string_lossy()
            );
        }
    }

    /* Log the required layers */
    if !required.is_empty() {
        debug!("Required Vulkan layers:");
        for layer in &required {
            debug!(" - {}", CStr::from_ptr(*layer).to_string_lossy());
        }
    }

    Ok(required)
}
