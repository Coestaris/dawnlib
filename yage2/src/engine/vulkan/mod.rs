mod cfg;
mod device;
mod instance;
mod queue;

use crate::engine::graphics::Graphics;
use crate::engine::vulkan::device::get_physical_device;
use crate::engine::vulkan::instance::{get_instance_extensions, get_layers, setup_debug};
use crate::engine::vulkan::queue::{get_device_extensions, get_queue_family_index};
use ash::{vk, Instance};
use log::{debug, info};
use std::ffi::c_char;

pub struct VulkanGraphicsInitArgs<'a> {
    pub(crate) instance_extensions: Vec<*const c_char>,
    pub(crate) device_extensions: Vec<*const c_char>,
    pub(crate) layers: Vec<*const c_char>,
    pub(crate) surface_constructor:
        Box<dyn Fn(&ash::Entry, &Instance) -> Result<vk::SurfaceKHR, VulkanGraphicsError> + 'a>,
}

struct Frame {
    index: usize,

    vk_image: vk::Image,
    vk_image_view: vk::ImageView,

    /* Currently using a single command buffer per frame/pool */
    command_buffer: vk::CommandBuffer,
    command_pool: vk::CommandPool,

    semaphore: vk::Semaphore,
    fence: vk::Fence,
}

struct VulkanObjects {
    entry: ash::Entry,
    instance: Instance,
    device: ash::Device,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
    swapchain: vk::SwapchainKHR,

    swapchain_loader: ash::khr::swapchain::Device,
}

pub struct VulkanGraphics {
    vk: VulkanObjects,
    frames: Vec<Frame>,
    current_frame_index: usize,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum VulkanGraphicsError {
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
    CreateCommandPoolFailed(vk::Result),
    EnumerateSurfaceCapabilitiesError(vk::Result),
    CreateCommandBufferFailed(vk::Result),
    CreateSemaphoreFailed(vk::Result),
    CreateFenceFailed(vk::Result),
}

#[allow(deprecated)]
impl Drop for VulkanGraphics {
    fn drop(&mut self) {
        unsafe {
            let _ = self.vk.device.device_wait_idle();

            /* We must destroy thing in reverse order of creation. */
            for frame in self.frames.iter_mut() {
                debug!(
                    "Destroying Vulkan command buffer: {:?}",
                    frame.command_buffer
                );
                self.vk
                    .device
                    .free_command_buffers(frame.command_pool, &[frame.command_buffer]);
                debug!("Destroying Vulkan command pool: {:?}", frame.command_pool);
                self.vk
                    .device
                    .destroy_command_pool(frame.command_pool, None);
            }

            for frame in self.frames.drain(..) {
                debug!("Destroying Vulkan image view: {:?}", frame.vk_image_view);
                self.vk.device.destroy_image_view(frame.vk_image_view, None);
            }

            /* Note: Images are not explicitly destroyed in Vulkan, they are managed by the swapchain. */

            if self.vk.swapchain != vk::SwapchainKHR::null() {
                debug!("Destroying Vulkan swapchain");
                self.vk
                    .swapchain_loader
                    .destroy_swapchain(self.vk.swapchain, None);
            }

            if self.vk.surface != vk::SurfaceKHR::null() {
                debug!("Destroying Vulkan surface");
                let surface_loader =
                    ash::khr::surface::Instance::new(&self.vk.entry, &self.vk.instance);
                surface_loader.destroy_surface(self.vk.surface, None);
            }

            if cfg!(debug_assertions) {
                if self.vk.debug_report_callback != vk::DebugReportCallbackEXT::null() {
                    debug!("Destroying Vulkan debug report callback");
                    let debug_report_loader =
                        ash::ext::debug_report::Instance::new(&self.vk.entry, &self.vk.instance);
                    debug_report_loader
                        .destroy_debug_report_callback(self.vk.debug_report_callback, None);
                }

                if self.vk.debug_messenger != vk::DebugUtilsMessengerEXT::null() {
                    debug!("Destroying Vulkan debug utils messenger");
                    let debug_utils_loader =
                        ash::ext::debug_utils::Instance::new(&self.vk.entry, &self.vk.instance);
                    debug_utils_loader.destroy_debug_utils_messenger(self.vk.debug_messenger, None);
                }
            }

            debug!("Destroying Vulkan device");
            self.vk.device.destroy_device(None);

            debug!("Destroying Vulkan instance");
            self.vk.instance.destroy_instance(None);
        }
    }
}

impl Graphics for VulkanGraphics {
    type Error = VulkanGraphicsError;
    type InitArgs<'a> = VulkanGraphicsInitArgs<'a>;

    fn new(init: VulkanGraphicsInitArgs<'_>) -> Result<Self, Self::Error>
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

            let (debug_report_callback, debug_messenger) =
                setup_debug(&entry, &instance, &instance_extensions)?;

            debug!("Creating Vulkan device");
            let physical_device = get_physical_device(&instance)?;
            let queue_family_index = get_queue_family_index(&instance, physical_device)?;
            let queue_priority = [1.0f32];
            let queue_create_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&queue_priority);
            let device_extensions = get_device_extensions(&instance, physical_device, &init)?;
            let device_extensions_array = device_extensions.as_slice();
            let device_create_info = vk::DeviceCreateInfo::default()
                .enabled_extension_names(device_extensions_array)
                .queue_create_infos(std::slice::from_ref(&queue_create_info));
            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(VulkanGraphicsError::EnumerateQueueFamiliesError)?;
            let surface = (init.surface_constructor)(&entry, &instance)?;
            let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(VulkanGraphicsError::EnumerateSurfaceCapabilitiesError)?;
            debug!("Surface capabilities: {:?}", surface_capabilities);

            debug!("Creating Vulkan swapchain");
            let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(surface_capabilities.min_image_count)
                .image_format(vk::Format::B8G8R8A8_SRGB)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_array_layers(surface_capabilities.max_image_array_layers)
                .image_extent(vk::Extent2D {
                    width: surface_capabilities.current_extent.width,
                    height: surface_capabilities.current_extent.height,
                })
                .image_usage(
                    vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                )
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(surface_capabilities.supported_composite_alpha)
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true);
            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .map_err(VulkanGraphicsError::CreateSwapchainFailed)?;
            let swapchain_images = swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(VulkanGraphicsError::GetSwapchainImagesFailed)?;
            let mut frames = Vec::with_capacity(swapchain_images.len());
            for image in &swapchain_images {
                debug!("Creating image view for swapchain image: {:?}", image);
                let image_view_create_info = vk::ImageViewCreateInfo::default()
                    .image(*image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_SRGB)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                let image_view = device
                    .create_image_view(&image_view_create_info, None)
                    .map_err(VulkanGraphicsError::CreateImageViewFailed)?;

                debug!("Created command pool for frame: {:?}", frames.len());
                let command_pool_create_info = vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family_index as u32)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
                let command_pool = device
                    .create_command_pool(&command_pool_create_info, None)
                    .map_err(VulkanGraphicsError::CreateCommandPoolFailed)?;

                debug!("Allocating command buffer for frame: {:?}", frames.len());
                let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);
                let command_buffers = device
                    .allocate_command_buffers(&command_buffer_allocate_info)
                    .map_err(VulkanGraphicsError::CreateCommandBufferFailed)?;
                let command_buffer = command_buffers[0];

                debug!("Creating semaphore for frame: {:?}", frames.len());
                let semaphore_create_info = vk::SemaphoreCreateInfo::default();
                let semaphore = device
                    .create_semaphore(&semaphore_create_info, None)
                    .map_err(VulkanGraphicsError::CreateSemaphoreFailed)?;

                debug!("Creating fence for frame: {:?}", frames.len());
                let fence_create_info =
                    vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
                let fence = device
                    .create_fence(&fence_create_info, None)
                    .map_err(VulkanGraphicsError::CreateFenceFailed)?;

                frames.push(Frame {
                    index: frames.len(),
                    vk_image: *image,
                    vk_image_view: image_view,
                    command_buffer,
                    command_pool,
                    semaphore,
                    fence,
                })
            }

            info!("Vulkan device created successfully");

            Ok(VulkanGraphics {
                vk: VulkanObjects {
                    entry,
                    instance,
                    device,
                    surface,
                    physical_device,
                    debug_messenger,
                    debug_report_callback,
                    swapchain,
                    swapchain_loader,
                },
                frames,
                current_frame_index: 0,
            })
        }
    }

    fn draw(&mut self) -> Result<(), Self::Error> {
        unsafe {
            let frame = self.get_current_frame();

            /*
            let fences = [frame.fence];
            self.vk
                .device
                .wait_for_fences(&fences, true, u64::MAX)
                .unwrap();
            self.vk.device.reset_fences(&fences).unwrap();

            let image_index = self
                .vk
                .swapchain_loader
                .acquire_next_image(
                    self.vk.swapchain,
                    u64::MAX,
                    frame.semaphore,
                    vk::Fence::null(),
                )
                .unwrap();

            self.vk
                .device
                .reset_command_buffer(
                    frame.command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .unwrap();

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.vk
                .device
                .begin_command_buffer(frame.command_buffer, &command_buffer_begin_info)
                .unwrap();

            let clear_value = vk::ClearColorValue {
                float32: [0.0, 1.0, 0.0, 1.0],
            };
            let clear_values = [vk::ClearValue { color: clear_value }];
*/
            Ok(())
        }
    }
}

impl VulkanGraphics {
    fn get_current_frame(&mut self) -> &mut Frame {
        &mut self.frames[self.current_frame_index]
    }
}
