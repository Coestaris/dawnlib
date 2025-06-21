use crate::engine::vulkan::{VkObject, VulkanGraphicsError, SURFACE_EXTENSION_NAME, WIN32_SURFACE_EXTENSION_NAME, XLIB_SURFACE_EXTENSION_NAME};
use ash::vk::{SurfaceKHR, Win32SurfaceCreateInfoKHR, HINSTANCE, HWND};
use ash::{vk, Device, Instance};
use log::debug;
use std::ffi::c_char;

pub(crate) struct Surface {
    pub(crate) vk_surface: SurfaceKHR,

    surface_loader: ash::khr::surface::Instance,

    name: Option<String>,
}

impl Surface {
    #[cfg(target_os = "windows")]
    pub fn new(
        entry: &ash::Entry,
        instance: &Instance,
        hinstance: HINSTANCE,
        hwnd: HWND,
        name: Option<String>,
    ) -> Result<Self, VulkanGraphicsError> {
        debug!("Creating surface with name: {:?}", name);

        let surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
        let create_info = Win32SurfaceCreateInfoKHR::default()
            .hinstance(hinstance)
            .hwnd(hwnd);

        let vk_surface = unsafe {
            surface_loader
                .create_win32_surface(&create_info, None)
                .map_err(VulkanGraphicsError::SurfaceCreateError)?
        };

        Ok(Surface {
            vk_surface,
            surface_loader: ash::khr::surface::Instance::new(entry, instance),
            name,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn new(
        entry: &ash::Entry,
        instance: &Instance,
        xlib_display: *mut std::ffi::c_void,
        xlib_window: u64,
        name: Option<String>,
    ) -> Result<Self, VulkanGraphicsError> {
        debug!("Creating surface with name: {:?}", name);

        let surface_loader = ash::khr::xlib_surface::Instance::new(entry, instance);
        let create_info = ash::khr::xlib_surface::XlibSurfaceCreateInfoKHR {
            dpy: xlib_display,
            window: xlib_window,
            ..Default::default()
        };

        let vk_surface = unsafe {
            surface_loader
                .create_xlib_surface(&create_info, None)
                .map_err(VulkanGraphicsError::SurfaceCreateError)?
        };

        Ok(Surface {
            vk_surface,
            surface_loader: ash::khr::surface::Instance::new(entry, instance),
            name,
        })
    }

    pub fn get_current_extent(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::Extent2D, VulkanGraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(VulkanGraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.current_extent)
    }

    pub fn get_min_images_count(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<u32, VulkanGraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(VulkanGraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.min_image_count)
    }

    pub fn get_current_transform(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceTransformFlagsKHR, VulkanGraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(VulkanGraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.current_transform)
    }
}

impl VkObject for Surface {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_surface".to_string())
    }

    fn destroy(&self, _: &Instance, _: &Device) -> Result<(), VulkanGraphicsError> {
        debug!(
            "Destroying surface: {} ({:?})",
            self.name(),
            self.vk_surface
        );
        unsafe {
            self.surface_loader.destroy_surface(self.vk_surface, None);
        }
        Ok(())
    }

    fn required_device_extensions() -> Vec<*const c_char> {
        vec![
            SURFACE_EXTENSION_NAME,
            #[cfg(target_os = "windows")]
            WIN32_SURFACE_EXTENSION_NAME,
            #[cfg(target_os = "linux")]
            XLIB_SURFACE_EXTENSION_NAME,
        ]
    }
}
