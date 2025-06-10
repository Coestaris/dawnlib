mod cfg;
mod device;
mod instance;
mod queue;

use crate::graphics::graphics::Graphics;
use crate::graphics::vulkan::cfg::{DEBUG_REPORT_EXTENSION_NAME, DEBUG_UTILS_EXTENSION_NAME};
use crate::graphics::vulkan::device::get_physical_device;
use crate::graphics::vulkan::instance::{get_instance_extensions, get_layers};
use crate::graphics::vulkan::queue::{get_device_extensions, get_queue_family_index};
use crate::utils::contains;
use ash::vk::{Handle, PhysicalDeviceFeatures};
use ash::{vk, Instance};
use log::{debug, error, info, trace, warn};
use std::ffi::{c_char, c_void, CStr};

pub struct VulkanGraphicsInitArgs {
    pub(crate) instance_extensions: Vec<*const c_char>,
    pub(crate) device_extensions: Vec<*const c_char>,
    pub(crate) layers: Vec<*const c_char>,
}

pub struct VulkanGraphics {
    pub(crate) entry: ash::Entry,
    pub(crate) instance: Instance,
    device: ash::Device,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    swapchain: vk::SwapchainKHR,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum VulkanGraphicsError {
    EntryCreationError(ash::LoadingError),
    EnumerateVersionError(vk::Result),
    InstanceCreationError(vk::Result),
    EnumerateLayersError(vk::Result),
    EnumerateExtensionsError(vk::Result),
    EnumeratePhysicalDevicesError(vk::Result),
    EnumerateQueueFamiliesError(vk::Result),
    CreateDebugMessengerFailed(vk::Result),
    CreateDebugReportFailed(vk::Result),
    CreateSwapchainFailed(vk::Result),
    EnumerateSurfaceCapabilitiesError(vk::Result),
}

pub(crate) trait VulkanGraphicsInternal {
    fn update_surface(&mut self, surface: vk::SurfaceKHR) -> Result<(), VulkanGraphicsError>;
}

impl Drop for VulkanGraphics {
    fn drop(&mut self) {
        unsafe {
            if self.surface != vk::SurfaceKHR::null() {
                debug!("Destroying Vulkan surface");
                let surface_loader = ash::khr::surface::Instance::new(&self.entry, &self.instance);
                surface_loader.destroy_surface(self.surface, None);
            }

            if self.swapchain != vk::SwapchainKHR::null() {
                debug!("Destroying Vulkan swapchain");
                let swapchain_loader =
                    ash::khr::swapchain::Device::new(&self.instance, &self.device);
                swapchain_loader.destroy_swapchain(self.swapchain, None);
            }

            if cfg!(debug_assertions) {
                if self.debug_report_callback != vk::DebugReportCallbackEXT::null() {
                    debug!("Destroying Vulkan debug report callback");
                    let debug_report_loader =
                        ash::ext::debug_report::Instance::new(&self.entry, &self.instance);
                    debug_report_loader
                        .destroy_debug_report_callback(self.debug_report_callback, None);
                }

                if self.debug_messenger != vk::DebugUtilsMessengerEXT::null() {
                    debug!("Destroying Vulkan debug utils messenger");
                    let debug_utils_loader =
                        ash::ext::debug_utils::Instance::new(&self.entry, &self.instance);
                    debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
                }
            }

            debug!("Destroying Vulkan device");
            self.device.destroy_device(None);

            debug!("Destroying Vulkan instance");
            self.instance.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn dd(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    p_user_data: *mut c_void,
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

unsafe extern "system" fn ee(
    flags: vk::DebugReportFlagsEXT,
    object_type: vk::DebugReportObjectTypeEXT,
    object: u64,
    location: usize,
    message_code: i32,
    p_layer_prefix: *const c_char,
    p_message: *const c_char,
    p_user_data: *mut c_void,
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

impl Graphics for VulkanGraphics {
    type Error = VulkanGraphicsError;
    type InitArgs = VulkanGraphicsInitArgs;

    fn new(init: VulkanGraphicsInitArgs) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        unsafe {
            debug!("Creating ASH entry");
            let entry = ash::Entry::load().map_err(VulkanGraphicsError::EntryCreationError)?;

            debug!("Enumerating Vulkan instance version");
            let vulkan_version = entry
                .try_enumerate_instance_version()
                .map_err(VulkanGraphicsError::EnumerateVersionError)?
                .unwrap_or(vk::make_api_version(0, 1, 0, 0));

            info!(
                "Supported Vulkan version: {}.{}.{}",
                vk::api_version_major(vulkan_version),
                vk::api_version_minor(vulkan_version),
                vk::api_version_patch(vulkan_version)
            );

            debug!("Creating Vulkan instance with version: {}", vulkan_version);
            let app_info = vk::ApplicationInfo {
                api_version: vulkan_version,
                ..Default::default()
            };
            let instance_extensions = get_instance_extensions(&entry, &init)?;
            let instance_extensions_array = instance_extensions.as_slice();

            let layers = get_layers(&entry, &init)?;
            let layers_array = layers.as_slice();

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(layers_array)
                .enabled_extension_names(instance_extensions_array);
            let instance = entry
                .create_instance(&create_info, None)
                .map_err(VulkanGraphicsError::InstanceCreationError)?;

            let mut debug_messenger = vk::DebugUtilsMessengerEXT::null();
            let mut debug_report_callback = vk::DebugReportCallbackEXT::null();

            if cfg!(debug_assertions) {
                if contains(&instance_extensions, DEBUG_REPORT_EXTENSION_NAME) {
                    debug!("Debug Report extension is enabled");
                    let debug_report_loader =
                        ash::ext::debug_report::Instance::new(&entry, &instance);
                    let debug_report_create_info = vk::DebugReportCallbackCreateInfoEXT::default()
                        .flags(
                            vk::DebugReportFlagsEXT::ERROR
                                | vk::DebugReportFlagsEXT::WARNING
                                | vk::DebugReportFlagsEXT::PERFORMANCE_WARNING
                                | vk::DebugReportFlagsEXT::DEBUG
                                | vk::DebugReportFlagsEXT::INFORMATION,
                        )
                        .pfn_callback(Some(ee));
                    debug_report_callback = debug_report_loader
                        .create_debug_report_callback(&debug_report_create_info, None)
                        .map_err(VulkanGraphicsError::CreateDebugReportFailed)?;
                } else if contains(&instance_extensions, DEBUG_UTILS_EXTENSION_NAME) {
                    debug!("Debug Utils extension is enabled");
                    let debug_utils_loader =
                        ash::ext::debug_utils::Instance::new(&entry, &instance);
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
                        .pfn_user_callback(Some(dd));
                    debug_messenger = debug_utils_loader
                        .create_debug_utils_messenger(&debug_utils_create_info, None)
                        .map_err(VulkanGraphicsError::CreateDebugMessengerFailed)?;
                } else {
                    warn!("None of the debug extensions are enabled. Debugging will be limited.");
                }
            }

            debug!("Creating Vulkan device");
            let physical_device = get_physical_device(&instance)?;
            let queue_family_index = get_queue_family_index(&instance, physical_device)?;

            let queue_priority = [1.0f32];
            let queue_create_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&queue_priority);

            let device_extensions =
                get_device_extensions(&entry, &instance, physical_device, &init)?;
            let device_extensions_array = device_extensions.as_slice();

            let device_create_info = vk::DeviceCreateInfo::default()
                .enabled_extension_names(device_extensions_array)
                .queue_create_infos(std::slice::from_ref(&queue_create_info));
            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(VulkanGraphicsError::EnumerateQueueFamiliesError)?;

            info!("Vulkan device created successfully");

            Ok(VulkanGraphics {
                entry,
                instance,
                device,
                physical_device,
                surface: vk::SurfaceKHR::null(),
                swapchain: vk::SwapchainKHR::null(),
                debug_messenger,
                debug_report_callback,
            })
        }
    }
}

impl VulkanGraphicsInternal for VulkanGraphics {
    fn update_surface(&mut self, surface: vk::SurfaceKHR) -> Result<(), VulkanGraphicsError> {
        unsafe {
            debug!("Updating Vulkan surface: {:?}", surface);
            self.surface = surface;

            let surface_loader = ash::khr::surface::Instance::new(&self.entry, &self.instance);
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
                .map_err(VulkanGraphicsError::EnumerateSurfaceCapabilitiesError)?;
            debug!("Surface capabilities: {:?}", surface_capabilities);

            debug!("Creating Vulkan swapchain");
            let swapchain_loader = ash::khr::swapchain::Device::new(&self.instance, &self.device);
            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(self.surface)
                .min_image_count(surface_capabilities.min_image_count)
                .image_format(vk::Format::B8G8R8A8_SRGB)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(vk::Extent2D {
                    width: surface_capabilities.current_extent.width,
                    height: surface_capabilities.current_extent.height,
                })
                .image_usage(vk::ImageUsageFlags::TRANSFER_DST)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(surface_capabilities.supported_composite_alpha)
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true);

            self.swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .map_err(VulkanGraphicsError::CreateSwapchainFailed)?;
        }
        Ok(())
    }
}
