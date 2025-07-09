use crate::vulkan::{GraphicsError, VkObject, SURFACE_EXTENSION_NAME};
use ash::vk::SurfaceKHR;
use ash::{vk, Device, Instance};
use log::debug;
use std::ffi::c_char;

#[cfg(target_os = "macos")]
use crate::vulkan::MACOS_SURFACE_EXTENSION_NAME;
#[cfg(target_os = "windows")]
use crate::vulkan::WIN32_SURFACE_EXTENSION_NAME;
#[cfg(target_os = "linux")]
use crate::vulkan::XLIB_SURFACE_EXTENSION_NAME;

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
        hinstance: vk::HINSTANCE,
        hwnd: vk::HWND,
        name: Option<String>,
    ) -> Result<Self, GraphicsError> {
        debug!("Creating surface with name: {:?}", name);

        let surface_loader = ash::khr::win32_surface::Instance::new(entry, instance);
        let create_info = ash::vk::Win32SurfaceCreateInfoKHR::default()
            .hinstance(hinstance)
            .hwnd(hwnd);

        let vk_surface = unsafe {
            surface_loader
                .create_win32_surface(&create_info, None)
                .map_err(GraphicsError::SurfaceCreateError)?
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
        xlib_display: *mut vk::Display,
        xlib_window: vk::Window,
        name: Option<String>,
    ) -> Result<Self, GraphicsError> {
        debug!("Creating surface with name: {:?}", name);

        let surface_loader = ash::khr::xlib_surface::Instance::new(entry, instance);
        let create_info = vk::XlibSurfaceCreateInfoKHR::default()
            .dpy(xlib_display)
            .window(xlib_window);

        let vk_surface = unsafe {
            surface_loader
                .create_xlib_surface(&create_info, None)
                .map_err(GraphicsError::SurfaceCreateError)?
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
    ) -> Result<vk::Extent2D, GraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(GraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.current_extent)
    }

    pub fn get_min_image_count(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<u32, GraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(GraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.min_image_count)
    }

    pub fn get_current_transform(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceTransformFlagsKHR, GraphicsError> {
        let surface_capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.vk_surface)
                .map_err(GraphicsError::SurfaceGetCapabilitiesError)?
        };

        Ok(surface_capabilities.current_transform)
    }

    pub fn supports_queue_family(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<bool, GraphicsError> {
        let supports = unsafe {
            self.surface_loader
                .get_physical_device_surface_support(
                    physical_device,
                    queue_family_index,
                    self.vk_surface,
                )
                .map_err(GraphicsError::SurfaceGetSupportError)?
        };

        Ok(supports)
    }
}

impl VkObject for Surface {
    fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| "unnamed_surface".to_string())
    }

    fn destroy(&self, _: &Instance, _: &Device) -> Result<(), GraphicsError> {
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

    fn required_instance_extensions() -> Vec<*const c_char> {
        vec![
            SURFACE_EXTENSION_NAME,
            #[cfg(target_os = "windows")]
            WIN32_SURFACE_EXTENSION_NAME,
            #[cfg(target_os = "linux")]
            XLIB_SURFACE_EXTENSION_NAME,
        ]
    }
}
