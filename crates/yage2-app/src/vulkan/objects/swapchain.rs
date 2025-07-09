use crate::vulkan::objects::surface::Surface;
use crate::vulkan::objects::sync::Semaphore;
use crate::vulkan::{VkObject, GraphicsError, SWAPCHAIN_EXTENSION_NAME};
use ash::{vk, Device, Instance};
use log::debug;
use std::ffi::c_char;

pub(crate) struct Swapchain {
    // Objects that are created by the swapchain
    pub(crate) vk_swapchain: vk::SwapchainKHR,
    pub(crate) vk_framebuffers: Vec<vk::Framebuffer>,
    pub(crate) vk_image_views: Vec<vk::ImageView>,

    // Objects that are used to create the swapchain
    vk_physical_device: vk::PhysicalDevice,
    vk_render_pass: vk::RenderPass,

    // Additional properties
    extent: vk::Extent2D,
    swapchain_loader: ash::khr::swapchain::Device,
    name: Option<String>,
}

impl Swapchain {
    pub fn new(
        _: &ash::Entry,
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        render_pass: vk::RenderPass,
        name: Option<String>,
    ) -> Result<Self, GraphicsError> {
        debug!("Creating swapchain {:?}", name);

        Ok(Swapchain {
            vk_swapchain: Default::default(),
            vk_framebuffers: vec![],
            vk_image_views: vec![],
            extent: Default::default(),
            vk_physical_device: physical_device,
            vk_render_pass: render_pass,
            name,
            swapchain_loader: ash::khr::swapchain::Device::new(&instance, &device),
        })
    }

    pub fn update(
        &mut self,
        instance: &Instance,
        device: &Device,
        surface: &Surface,
    ) -> Result<(), GraphicsError> {
        // Wait for the device to be idle before updating the swapchain
        unsafe { device.device_wait_idle() }.map_err(GraphicsError::DeviceWaitIdleError)?;

        // Destroy the old swapchain and its resources
        self.destroy(instance, device)?;

        let extent = surface.get_current_extent(self.vk_physical_device)?;

        // Create a new swapchain
        const DEFAULT_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;
        let images_count = surface.get_min_image_count(self.vk_physical_device)?;
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface.vk_surface)
            .min_image_count(images_count)
            .image_format(DEFAULT_FORMAT)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(surface.get_current_transform(self.vk_physical_device)?)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true);
        let vk_swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .map_err(GraphicsError::SwapchainCreateError)?
        };

        let vk_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(vk_swapchain)
                .map_err(GraphicsError::SwapchainGetImagesError)?
        };

        let mut vk_image_views = Vec::new();
        for i in 0..images_count {
            debug!("Creating image view for image index: {}", i);

            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(vk_images[i as usize])
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(DEFAULT_FORMAT)
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
            let vk_image_view = unsafe {
                device
                    .create_image_view(&image_view_create_info, None)
                    .map_err(GraphicsError::ImageViewCreateError)?
            };

            vk_image_views.push(vk_image_view);
        }

        let mut vk_framebuffers = Vec::new();
        for image_view in vk_image_views.iter() {
            let image_views = vec![*image_view];
            let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                .render_pass(self.vk_render_pass)
                .attachments(&image_views)
                .width(extent.width)
                .height(extent.height)
                .layers(1);

            let vk_framebuffer = unsafe {
                device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .map_err(GraphicsError::FramebufferCreateError)?
            };
            vk_framebuffers.push(vk_framebuffer);
        }

        self.vk_swapchain = vk_swapchain;
        self.vk_framebuffers = vk_framebuffers;
        self.vk_image_views = vk_image_views;
        self.extent = extent;

        debug!(
            "Swapchain created with {} images, extent: {:?}",
            self.vk_image_views.len(),
            self.extent
        );
        Ok(())
    }

    pub fn acquire_next_image(
        &self,
        _: &Device,
        semaphore: &Semaphore,
    ) -> Result<u32, GraphicsError> {
        let acquire_info = vk::AcquireNextImageInfoKHR::default()
            .swapchain(self.vk_swapchain)
            .timeout(u64::MAX)
            .device_mask(1)
            .semaphore(semaphore.handle())
            .fence(vk::Fence::null());

        match unsafe { self.swapchain_loader.acquire_next_image2(&acquire_info) } {
            Ok((index, false)) => Ok(index),
            Ok((_, true)) => Err(GraphicsError::SwapchainSuboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(GraphicsError::SwapchainSuboptimal),
            Err(e) => Err(GraphicsError::SwapchainAcquireNextImageError(e)),
        }
    }

    pub fn queue_present(
        &self,
        _: &Device,
        queue: vk::Queue,
        image_index: u32,
        semaphore: &Semaphore,
    ) -> Result<(), GraphicsError> {
        let indices = [image_index];
        let semaphores = [semaphore.handle()];
        let swapchains = [self.vk_swapchain];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .wait_semaphores(&semaphores)
            .image_indices(&indices);
        match unsafe { self.swapchain_loader.queue_present(queue, &present_info) } {
            Ok(false) => Ok(()),
            Ok(true) => Err(GraphicsError::SwapchainSuboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Err(GraphicsError::SwapchainSuboptimal),
            Err(e) => Err(GraphicsError::SwapchainQueuePresentError(e)),
        }
    }

    pub fn get_extent(&self) -> vk::Extent2D {
        self.extent
    }

    pub fn get_images_count(&self) -> usize {
        self.vk_image_views.len()
    }
}

impl VkObject for Swapchain {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_swapchain".to_string())
    }

    fn destroy(&self, _: &Instance, device: &Device) -> Result<(), GraphicsError> {
        debug!(
            "Destroying swapchain: {} ({:?})",
            self.name(),
            self.vk_swapchain
        );

        for framebuffer in &self.vk_framebuffers {
            unsafe {
                device.destroy_framebuffer(*framebuffer, None);
            }
        }
        for image_view in &self.vk_image_views {
            unsafe {
                device.destroy_image_view(*image_view, None);
            }
        }

        if self.vk_swapchain != vk::SwapchainKHR::null() {
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(self.vk_swapchain, None);
            }
        }

        Ok(())
    }

    fn required_device_extensions() -> Vec<*const c_char> {
        vec![SWAPCHAIN_EXTENSION_NAME]
    }
}
