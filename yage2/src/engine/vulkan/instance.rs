use crate::engine::vulkan::{VulkanGraphicsError, DEBUG_REPORT_EXTENSION_NAME, DEBUG_UTILS_EXTENSION_NAME};
use crate::core::utils::contains;
use ash::vk;
use log::{debug, error, info, trace, warn};
use std::ffi::{c_char, c_void, CStr};
use crate::engine::vulkan::graphics::VulkanGraphicsInitArgs;
use crate::engine::vulkan::objects::{get_required_instance_extensions, get_required_layers, get_wanted_instance_extensions, get_wanted_layers};

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

unsafe extern "system" fn vulkan_message_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _: *mut c_void,
) -> vk::Bool32 {
    let callback_data = &*p_callback_data;
    let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        error!("[VK][{:?}]: {}", message_types, message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        warn!("[VK][{:?}]: {}", message_types, message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
        info!("[VK][{:?}]: {}", message_types, message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) {
        trace!("[VK][{:?}]: {}", message_types, message);
    }

    vk::FALSE
}

unsafe extern "system" fn vulkan_message_callback_ext(
    flags: vk::DebugReportFlagsEXT,
    object_type: vk::DebugReportObjectTypeEXT,
    object: u64,
    location: usize,
    message_code: i32,
    p_layer_prefix: *const c_char,
    p_message: *const c_char,
    _: *mut c_void,
) -> vk::Bool32 {
    let message = CStr::from_ptr(p_message).to_string_lossy();
    let layer_prefix = if p_layer_prefix.is_null() {
        "Unknown Layer"
    } else {
        &*CStr::from_ptr(p_layer_prefix).to_string_lossy()
    };

    if flags.contains(vk::DebugReportFlagsEXT::ERROR) {
        error!(
            "[VK][{}][{:?},0x{:x},0x{:x}] {} ({})",
            layer_prefix, object_type, object, location, message, message_code
        );
    } else if flags.contains(vk::DebugReportFlagsEXT::WARNING) {
        warn!(
            "[VK][{}][{:?},0x{:x},0x{:x}] {} ({})",
            layer_prefix, object_type, object, location, message, message_code
        );
    } else if flags.contains(vk::DebugReportFlagsEXT::PERFORMANCE_WARNING) {
        warn!(
            "[VK PERF][{}][{:?},0x{:x},0x{:x}] {} ({})",
            layer_prefix, object_type, object, location, message, message_code
        );
    } else if flags.contains(vk::DebugReportFlagsEXT::DEBUG) {
        debug!(
            "[VK][{}][{:?},0x{:x},0x{:x}] {} ({})",
            layer_prefix, object_type, object, location, message, message_code
        );
    } else if flags.contains(vk::DebugReportFlagsEXT::INFORMATION) {
        trace!(
            "[VK][{}][{:?},0x{:x},0x{:x}] {} ({})",
            layer_prefix,
            object_type,
            object,
            location,
            message,
            message_code
        );
    }

    vk::FALSE
}

#[allow(deprecated)]
pub unsafe fn setup_debug(
    entry: &ash::Entry,
    instance: &ash::Instance,
    instance_extensions: &Vec<*const c_char>,
) -> Result<(vk::DebugReportCallbackEXT, vk::DebugUtilsMessengerEXT), VulkanGraphicsError> {
    let mut debug_messenger = vk::DebugUtilsMessengerEXT::null();
    let mut debug_report_callback = vk::DebugReportCallbackEXT::null();

    if cfg!(debug_assertions) {
        if contains(instance_extensions, DEBUG_REPORT_EXTENSION_NAME) {
            debug!("Debug Report extension is enabled");
            let debug_report_loader = ash::ext::debug_report::Instance::new(&entry, &instance);
            let debug_report_create_info = vk::DebugReportCallbackCreateInfoEXT::default()
                .flags(
                    vk::DebugReportFlagsEXT::ERROR
                        | vk::DebugReportFlagsEXT::WARNING
                        | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING
                        | vk::DebugReportFlagsEXT::DEBUG
                        | vk::DebugReportFlagsEXT::INFORMATION,
                )
                .pfn_callback(Some(vulkan_message_callback_ext));
            debug_report_callback = debug_report_loader
                .create_debug_report_callback(&debug_report_create_info, None)
                .map_err(VulkanGraphicsError::CreateDebugReportFailed)?;
        } else if contains(instance_extensions, DEBUG_UTILS_EXTENSION_NAME) {
            debug!("Debug Utils extension is enabled");
            let debug_utils_loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let debug_utils_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                        | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_message_callback));
            debug_messenger = debug_utils_loader
                .create_debug_utils_messenger(&debug_utils_create_info, None)
                .map_err(VulkanGraphicsError::CreateDebugMessengerFailed)?;
        } else {
            warn!("None of the debug extensions are enabled. Debugging will be limited.");
        }
    }

    Ok((debug_report_callback, debug_messenger))
}
