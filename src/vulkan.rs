use crate::window::Graphics;
use ash::{Instance, vk};
use log::{debug, info, trace, warn};
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
    device: ash::Device,
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
    EnumeratePhysicalDevicesError(vk::Result),
    EnumerateQueueFamiliesError(vk::Result),
}

pub(crate) trait VulkanGraphicsInternal {
    fn update_surface(&mut self, surface: vk::SurfaceKHR) -> Result<(), VulkanGraphicsError>;
}

unsafe fn get_supported_extensions(
    entry: &ash::Entry,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let extensions = entry
        .enumerate_instance_extension_properties(None)
        .map_err(VulkanGraphicsError::EnumerateExtensionsError)?;

    /* Log the available extensions */
    if !extensions.is_empty() {
        trace!("Available Vulkan extensions:");
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

unsafe fn get_extensions(
    supported: Vec<*const c_char>,
    init: &VulkanGraphicsInitArgs,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let required = vec![
        ash::khr::surface::NAME.as_ptr() as *const c_char,
        ash::ext::debug_utils::NAME.as_ptr() as *const c_char,
    ]
    .into_iter()
    .chain(init.extensions.iter().copied())
    .collect::<Vec<_>>();

    /* TODO: filter out unsupported extensions */

    /* log the required extensions */
    if !required.is_empty() {
        debug!("Required Vulkan extensions:");
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

unsafe fn get_layers(
    supported: Vec<*const c_char>,
    init: &VulkanGraphicsInitArgs,
) -> Result<Vec<*const c_char>, VulkanGraphicsError> {
    let mut required = init.layers.clone();

    /* If the validation layer is supported,
     * add it to the required layers */
    if supported.contains(&VALIDATION_LAYER.as_ptr()) {
        required.push(VALIDATION_LAYER.as_ptr() as *const c_char);
    } else {
        warn!(
            "Validation layer {} is not supported by the Vulkan implementation",
            VALIDATION_LAYER.to_string_lossy()
        );
    }

    /* TODO: filter out unsupported layers */

    /* Log the required layers */
    if !required.is_empty() {
        debug!("Required Vulkan layers:");
        for layer in &required {
            debug!(" - {}", CStr::from_ptr(*layer).to_string_lossy());
        }
    }

    Ok(required)
}

unsafe fn get_physical_device(
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
    info!("Using physical device: {:?}", device);

    Ok(device)
}

unsafe fn get_queue_family_index(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<usize, VulkanGraphicsError> {
    let queue_families = instance.get_physical_device_queue_family_properties(physical_device);

    if queue_families.is_empty() {
        return Err(VulkanGraphicsError::EnumerateQueueFamiliesError(
            vk::Result::ERROR_INITIALIZATION_FAILED,
        ));
    }

    /* Log the available queue families */
    trace!("Available Vulkan queue families:");
    for (i, family) in queue_families.iter().enumerate() {
        trace!(
            " - Queue Family {}: {:?} (count={})",
            i, family.queue_flags, family.queue_count
        );
    }

    /* TODO: Implement queue family selection logic */
    let family_index = 0;
    info!("Using queue family index: {}", family_index);

    Ok((family_index))
}

impl Drop for VulkanGraphics {
    fn drop(&mut self) {
        unsafe {
            debug!("Destroying Vulkan device");
            self.device.destroy_device(None);
            debug!("Destroying Vulkan instance");
            self.instance.destroy_instance(None);
        }
    }
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
            let supported_layers = get_supported_layers(&entry)?;
            let supported_extensions = get_supported_extensions(&entry)?;

            let extensions = get_extensions(supported_extensions, &init)?;
            let extensions_array = extensions.as_slice();

            let layers = get_layers(supported_layers, &init)?;
            let layers_array = layers.as_slice();

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(layers_array)
                .enabled_extension_names(extensions_array);
            let instance = entry
                .create_instance(&create_info, None)
                .map_err(VulkanGraphicsError::InstanceCreationError)?;

            debug!("Creating Vulkan device");
            let physical_device = get_physical_device(&instance)?;
            let queue_family_index = get_queue_family_index(&instance, physical_device)?;

            let queue_priority = [1.0f32];
            let queue_create_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index as u32)
                .queue_priorities(&queue_priority);
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_create_info));
            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(VulkanGraphicsError::EnumerateQueueFamiliesError)?;

            info!("Vulkan device created successfully");

            Ok(VulkanGraphics {
                entry,
                instance,
                device,
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
