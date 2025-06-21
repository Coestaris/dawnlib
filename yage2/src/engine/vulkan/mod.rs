mod device;
pub(crate) mod graphics;
mod instance;
pub(crate) mod objects;
mod queue;

use ash::{vk, Instance};
use std::ffi::{c_char, CStr};

#[derive(Debug)]
#[allow(dead_code)]
pub enum VulkanGraphicsError {
    UnknownError(vk::Result),
    EntryCreationError(ash::LoadingError),
    SurfaceCreateError(vk::Result),
    EnumerateVersionError(vk::Result),
    InstanceCreationError(vk::Result),
    EnumerateLayersError(vk::Result),
    EnumerateExtensionsError(vk::Result),
    EnumeratePhysicalDevicesError(vk::Result),
    EnumerateQueueFamiliesError(vk::Result),
    CreateDebugMessengerFailed(vk::Result),
    CreateDebugReportFailed(vk::Result),
    CreateSwapchainFailed(vk::Result),
    GetSwapchainImagesFailed(vk::Result),
    CreateImageViewFailed(vk::Result),
    ShaderModuleCreateError(vk::Result),
    ShaderFileReadError(String),
    CreateFramebufferFailed(vk::Result),
    RenderPassCreateError(vk::Result),
    PipelineCreateError(vk::Result),
    PipelineLayoutCreateError(vk::Result),

    // Command pool/buffer-related errors
    CommandPoolCreateFailed(vk::Result),
    CommandBufferAllocateFailed(vk::Result),
    CommandBufferResetFailed(vk::Result),
    CommandBufferBeginFailed(vk::Result),
    CommandBufferEndFailed(vk::Result),
    CommandBuffersFreeFailed(vk::Result),
    CommandPoolResetFailed(vk::Result),
    CommandPoolDestroyFailed(vk::Result),

    // Semaphore-related errors
    SemaphoreCreateFailed(vk::Result),
    SemaphoreDestroyFailed(vk::Result),

    // Fence-related errors
    FenceCreateFailed(vk::Result),
    FenceResetFailed(vk::Result),
    FenceWaitFailed(vk::Result),
    FenceDestroyFailed(vk::Result),

    // Surface-related errors
    SurfaceGetCapabilitiesError(vk::Result),
    SurfaceGetFormatsError(vk::Result),

    // Swapchain-related errors
    SwapchainCreateError(vk::Result),
    SwapchainGetImagesError(vk::Result),
    ImageViewCreateError(vk::Result),
    FramebufferCreateError(vk::Result),
    SwapchainAcquireNextImageError(vk::Result),
    SwapchainQueuePresentError(vk::Result),
    SwapchainSuboptimal,
}

pub(crate) const VALIDATION_LAYER_NAME: *const c_char =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") }.as_ptr()
        as *const c_char;
pub(crate) const SURFACE_EXTENSION_NAME: *const c_char = ash::khr::surface::NAME.as_ptr() as *const c_char;
pub(crate) const WIN32_SURFACE_EXTENSION_NAME: *const c_char =
    ash::khr::win32_surface::NAME.as_ptr() as *const c_char;
pub(crate) const XLIB_SURFACE_EXTENSION_NAME: *const c_char =
    ash::khr::xlib_surface::NAME.as_ptr() as *const c_char;
pub(crate) const SWAPCHAIN_EXTENSION_NAME: *const c_char = ash::khr::swapchain::NAME.as_ptr() as *const c_char;
pub(crate) const DYNAMIC_RENDERING_EXTENSION_NAME: *const c_char =
    ash::khr::dynamic_rendering::NAME.as_ptr() as *const c_char;
pub(crate) const SYNCHRONIZATION_2_EXTENSION_NAME: *const c_char =
    ash::khr::synchronization2::NAME.as_ptr() as *const c_char;
pub(crate) const DEBUG_UTILS_EXTENSION_NAME: *const c_char =
    ash::ext::debug_utils::NAME.as_ptr() as *const c_char;
pub(crate) const DEBUG_REPORT_EXTENSION_NAME: *const c_char =
    ash::ext::debug_report::NAME.as_ptr() as *const c_char;

pub(crate) trait VkObject {
    fn name(&self) -> String;
    fn destroy(&self, instance: &Instance, device: &ash::Device)
        -> Result<(), VulkanGraphicsError>;

    fn required_device_extensions() -> Vec<*const c_char> {
        vec![]
    }
    fn required_instance_extensions() -> Vec<*const c_char> {
        vec![]
    }
    fn required_layers() -> Vec<*const c_char> {
        vec![]
    }
    fn desired_device_extensions() -> Vec<*const c_char> {
        vec![]
    }
    fn desired_instance_extensions() -> Vec<*const c_char> {
        vec![]
    }
    fn desired_layers() -> Vec<*const c_char> {
        vec![]
    }
}
