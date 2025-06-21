use std::ffi::c_char;
use ash::{Device, Instance};
use crate::engine::vulkan::{VkObject, VulkanGraphicsError, DEBUG_REPORT_EXTENSION_NAME, DEBUG_UTILS_EXTENSION_NAME, DYNAMIC_RENDERING_EXTENSION_NAME, SYNCHRONIZATION_2_EXTENSION_NAME, VALIDATION_LAYER_NAME};
use crate::engine::vulkan::objects::command_buffer::CommandBuffer;
use crate::engine::vulkan::objects::shader::Shader;
use crate::engine::vulkan::objects::surface::Surface;
use crate::engine::vulkan::objects::swapchain::Swapchain;
use crate::engine::vulkan::objects::sync::{Fence, Semaphore};

pub(crate) mod sync;
pub(crate) mod swapchain;
pub(crate) mod surface;
pub(crate) mod command_buffer;
pub(crate) mod shader;

struct Placeholder;
impl VkObject for Placeholder {
    fn name(&self) -> String {
        todo!()
    }

    fn destroy(&self, instance: &Instance, device: &Device) -> Result<(), VulkanGraphicsError> {
        todo!()
    }

    fn desired_layers() -> Vec<*const c_char> {
        vec![
            VALIDATION_LAYER_NAME
        ]
    }

    fn required_device_extensions() -> Vec<*const c_char> {
        vec![
            DYNAMIC_RENDERING_EXTENSION_NAME,
            SYNCHRONIZATION_2_EXTENSION_NAME
        ]
    }
    
    fn desired_instance_extensions() -> Vec<*const c_char> {
        vec![
            DEBUG_UTILS_EXTENSION_NAME,
            DEBUG_REPORT_EXTENSION_NAME
        ]
    }
}   




/// Defines a functions that calls:
/// * required_device_extensions
///  * required_instance_extensions
///  * desired_device_extensions
///  * desired_instance_extensions
///  * required_layers
//   * desired_layers
/// on each of the provided types and concatenates 
/// the results into a single vector.
macro_rules! some_magic_macro {
    ($($type:ident),+) => {
        pub (crate) fn get_required_device_extensions() -> Vec<*const c_char> {
            let mut extensions = vec![];
            $(
                extensions.extend($type::required_device_extensions());
            )+
            extensions
        }

        pub (crate) fn get_required_instance_extensions() -> Vec<*const c_char> {
            let mut extensions = vec![];
            $(
                extensions.extend($type::required_instance_extensions());
            )+
            extensions
        }

        pub (crate) fn get_wanted_device_extensions() -> Vec<*const c_char> {
            let mut extensions = vec![];
            $(
                extensions.extend($type::desired_device_extensions());
            )+
            extensions
        }

        pub (crate) fn get_wanted_instance_extensions() -> Vec<*const c_char> {
            let mut extensions = vec![];
            $(
                extensions.extend($type::desired_instance_extensions());
            )+
            extensions
        }
        
        pub (crate) fn get_required_layers() -> Vec<*const c_char> {
            let mut layers = vec![];
            $(
                layers.extend($type::required_layers());
            )+
            layers
        }
        
        pub (crate) fn get_wanted_layers() -> Vec<*const c_char> {
            let mut layers = vec![];
            $(
                layers.extend($type::desired_layers());
            )+
            layers
        }
    };
}

some_magic_macro!(
    Semaphore,
    Shader,
    Fence,
    CommandBuffer,
    Surface,
    Swapchain
);