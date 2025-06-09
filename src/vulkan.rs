use crate::window::Graphics;
use ash::{Instance, vk};
use log::{debug, info, trace};
use std::ffi::{CStr, c_char};

const VALIDATION_LAYER: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };

pub struct VulkanGraphicsInitArgs {
    pub(crate) extensions: Vec<*const c_char>,
    pub(crate) layers: Vec<*const c_char>,
}

pub struct VulkanGraphics {
    pub(crate) entry: ash::Entry,
    pub(crate) instance: Instance,

    surface: vk::SurfaceKHR,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum VulkanGraphicsError {
    EntryCreationError(ash::LoadingError),
    EnumerateVersionError(vk::Result),
    InstanceCreationError(vk::Result),
    EnumerateLayersError(vk::Result),
    EnumerateExtensionsError(vk::Result),
}

pub(crate) trait VulkanGraphicsInternal {
    fn update_surface(&mut self, surface: vk::SurfaceKHR) -> Result<(), VulkanGraphicsError>;
}

impl Graphics for VulkanGraphics {
    type Error = VulkanGraphicsError;
    type InitArgs = VulkanGraphicsInitArgs;

    fn new(init: VulkanGraphicsInitArgs) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        unsafe {
            debug!("Creating Vulkan instance");
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

            let supported_layers = entry
                .enumerate_instance_layer_properties()
                .map_err(VulkanGraphicsError::EnumerateLayersError)?;
            trace!("Supported Vulkan layers:");
            for layer in supported_layers {
                trace!(
                    " - {}",
                    CStr::from_ptr(layer.layer_name.as_ptr()).to_string_lossy()
                );
            }

            let extensions = vec![
                ash::khr::surface::NAME.as_ptr() as *const c_char,
                ash::ext::debug_utils::NAME.as_ptr() as *const c_char,
            ]
            .into_iter()
            .chain(init.extensions)
            .collect::<Vec<_>>();
            debug!("Extensions to be enabled:");
            for ext in &extensions {
                debug!(" - {}", CStr::from_ptr(*ext).to_string_lossy());
            }
            let extensions: &[*const c_char] = extensions.as_slice();

            let layers = vec![].into_iter().chain(init.layers).collect::<Vec<_>>();
            debug!("Layers to be enabled:");
            for layer in &layers {
                debug!(" - {}", CStr::from_ptr(*layer).to_string_lossy());
            }
            let layers: &[*const c_char] = layers.as_slice();

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(layers)
                .enabled_extension_names(extensions);
            let instance = entry
                .create_instance(&create_info, None)
                .map_err(VulkanGraphicsError::InstanceCreationError)?;

            Ok(VulkanGraphics {
                entry,
                instance,
                surface: vk::SurfaceKHR::null(),
            })
        }
    }
}

impl VulkanGraphicsInternal for VulkanGraphics {
    fn update_surface(&mut self, surface: vk::SurfaceKHR) -> Result<(), VulkanGraphicsError> {
        self.surface = surface;
        Ok(())
    }
}
