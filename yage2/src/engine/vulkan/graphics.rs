use std::ffi::c_char;
use ash::{vk, Instance};
use ash::vk::{CommandBufferResetFlags, CommandPoolCreateFlags};
use log::{debug, info};
use crate::engine::graphics::{Graphics, TickResult};
use crate::engine::object::Renderable;
use crate::engine::vulkan::{objects, VulkanGraphicsError, VkObject};
use crate::engine::vulkan::device::{get_device_extensions, get_physical_device};
use crate::engine::vulkan::instance::{get_instance_extensions, get_layers, setup_debug};
use crate::engine::vulkan::objects::command_buffer::CommandBuffer;
use crate::engine::vulkan::objects::shader::{Shader, ShaderType};
use crate::engine::vulkan::objects::surface::Surface;
use crate::engine::vulkan::objects::swapchain::Swapchain;
use crate::engine::vulkan::objects::sync::{Fence, Semaphore};
use crate::engine::vulkan::queue::get_queue_family_index;

pub struct VulkanGraphicsInitArgs<'a> {
    pub(crate) instance_extensions: Vec<*const c_char>,
    pub(crate) device_extensions: Vec<*const c_char>,
    pub(crate) layers: Vec<*const c_char>,
    pub(crate) surface_constructor:
        Box<dyn Fn(&ash::Entry, &Instance) -> Result<Surface, VulkanGraphicsError> + 'a>,
}

struct Frame {
    index: usize,
    command_buffer: CommandBuffer,
    render_finished_semaphore: Semaphore,
    image_available_semaphore: Semaphore,
    in_flight_fence: Fence,
}

struct VulkanObjects {
    entry: ash::Entry,
    instance: Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
    pipeline: vk::Pipeline, // This should be the actual pipeline, not layout
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,

    surface: Surface,
    swapchain: Swapchain,
}

pub struct VulkanGraphics {
    vk: VulkanObjects,
    frames: Vec<Frame>,
    current_frame_index: usize,

    shader1: Option<Shader>,
    shader2: Option<Shader>,
}

#[allow(deprecated)]
impl Drop for VulkanGraphics {
    fn drop(&mut self) {
        unsafe {
            let _ = self.vk.device.device_wait_idle();

            debug!("Destroying Vulkan pipeline: {:?}", self.vk.pipeline);
            self.vk.device.destroy_pipeline(self.vk.pipeline, None);

            if let Some(shader1) = self.shader1.take() {
                let _ = shader1.destroy(&self.vk.instance, &self.vk.device);
            }

            if let Some(shader2) = self.shader2.take() {
                let _ = shader2.destroy(&self.vk.instance, &self.vk.device);
            }

            debug!(
                "Destroying Vulkan pipeline layout: {:?}",
                self.vk.pipeline_layout
            );
            self.vk
                .device
                .destroy_pipeline_layout(self.vk.pipeline_layout, None);

            debug!("Destroying Vulkan render pass: {:?}", self.vk.render_pass);
            self.vk
                .device
                .destroy_render_pass(self.vk.render_pass, None);

            /* We must destroy thing in reverse order of creation. */
            for frame in self.frames.iter_mut() {
                let _ = frame
                    .command_buffer
                    .destroy(&self.vk.instance, &self.vk.device);
                let _ = frame
                    .render_finished_semaphore
                    .destroy(&self.vk.instance, &self.vk.device);
                let _ = frame
                    .image_available_semaphore
                    .destroy(&self.vk.instance, &self.vk.device);
                let _ = frame
                    .in_flight_fence
                    .destroy(&self.vk.instance, &self.vk.device);
            }

            let _ = self
                .vk
                .swapchain
                .destroy(&self.vk.instance, &self.vk.device);
            let _ = self.vk.surface.destroy(&self.vk.instance, &self.vk.device);

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

impl VulkanGraphics {
    fn process_buffer(
        &self,
        frame: &Frame,
        renderables: &[Renderable],
    ) -> Result<TickResult, VulkanGraphicsError> {
        frame
            .command_buffer
            .reset(&self.vk.device, CommandBufferResetFlags::RELEASE_RESOURCES)?;
        frame.command_buffer.begin(
            &self.vk.device,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        )?;

        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.vk.render_pass)
            .framebuffer(self.vk.swapchain.vk_framebuffers[frame.index])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: 800,  // Replace with actual width
                    height: 600, // Replace with actual height
                },
            })
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }]);

        unsafe {
            self.vk.device.cmd_begin_render_pass(
                frame.command_buffer.vk_command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        unsafe {
            self.vk.device.cmd_bind_pipeline(
                frame.command_buffer.vk_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.vk.pipeline, // This should be the actual pipeline, not layout
            );
        }

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: 800.0,  // Replace with actual width
            height: 600.0, // Replace with actual height
            min_depth: 0.0,
            max_depth: 1.0,
        };
        unsafe {
            self.vk.device.cmd_set_viewport(
                frame.command_buffer.vk_command_buffer,
                0,
                std::slice::from_ref(&viewport),
            );
        }

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: 800,  // Replace with actual width
                height: 600, // Replace with actual height
            },
        };
        unsafe {
            self.vk.device.cmd_set_scissor(
                frame.command_buffer.vk_command_buffer,
                0,
                std::slice::from_ref(&scissor),
            );
        }

        // Here you would typically bind vertex buffers, index buffers, etc.

        frame.command_buffer.draw(
            &self.vk.device,
            3, // Number of vertices to draw, replace with actual count
            1, // Instance count, replace with actual if using instancing
            0, // First vertex
            0, // First instance
        );

        unsafe {
            self.vk
                .device
                .cmd_end_render_pass(frame.command_buffer.vk_command_buffer);
        }

        frame.command_buffer.end(&self.vk.device)?;

        Ok(TickResult {
            drawn_triangles: 1, // Placeholder, actual rendering logic would go here
        })
    }

    fn get_current_frame(&self) -> &Frame {
        &self.frames[self.current_frame_index]
    }
}

impl Graphics<VulkanGraphicsError> for VulkanGraphics {
    type InitArgs<'a> = VulkanGraphicsInitArgs<'a>;

    fn new(init: VulkanGraphicsInitArgs<'_>) -> Result<Self, VulkanGraphicsError>
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

            info!("Vulkan device created successfully");
            let shader1 = Shader::new_from_file(
                ShaderType::Vertex,
                &device,
                "D:\\Coding\\yage2\\app\\resources\\shaders\\triangle.vert.spv",
                Some("triangle_vert".to_string()),
            )?;
            let shader2 = Shader::new_from_file(
                ShaderType::Vertex,
                &device,
                "D:\\Coding\\yage2\\app\\resources\\shaders\\triangle.frag.spv",
                Some("triangle_frag".to_string()),
            )?;

            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state_create_info =
                vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
            let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::default();
            let input_assembly_state_create_info =
                vk::PipelineInputAssemblyStateCreateInfo::default()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    .primitive_restart_enable(false);
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: surface.get_current_extent(physical_device)?.width as f32,
                height: surface.get_current_extent(physical_device)?.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: surface.get_current_extent(physical_device)?,
            };
            let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::default()
                .viewports(std::slice::from_ref(&viewport))
                .scissors(std::slice::from_ref(&scissor));
            let rasterization_state_create_info =
                vk::PipelineRasterizationStateCreateInfo::default()
                    .depth_clamp_enable(false)
                    .rasterizer_discard_enable(false)
                    .polygon_mode(vk::PolygonMode::FILL)
                    .line_width(1.0)
                    .cull_mode(vk::CullModeFlags::BACK)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE);
            let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);
            let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::default()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .blend_enable(false);
            let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::default()
                .logic_op_enable(false)
                .attachments(std::slice::from_ref(&color_blend_attachment_state));
            let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default();
            let pipeline_layout = device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .map_err(VulkanGraphicsError::PipelineLayoutCreateError)?;

            let color_attachment_description = vk::AttachmentDescription::default()
                .format(vk::Format::B8G8R8A8_SRGB)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

            let color_attachment_reference = vk::AttachmentReference::default()
                .attachment(0)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            let subpass_create_info = vk::SubpassDescription::default()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(std::slice::from_ref(&color_attachment_reference))
                .input_attachments(&[])
                .preserve_attachments(&[]);

            let render_pass_create_info = vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color_attachment_description))
                .subpasses(std::slice::from_ref(&subpass_create_info));
            let render_pass = device
                .create_render_pass(&render_pass_create_info, None)
                .map_err(VulkanGraphicsError::RenderPassCreateError)?;

            let mut swapchain = Swapchain::new(
                &entry,
                &instance,
                &device,
                physical_device,
                render_pass,
                Some("main_swapchain".to_string()),
            )?;

            let mut frames = vec![];
            for i in 0..swapchain.get_images_count() {
                frames.push(Frame {
                    index: frames.len(),
                    command_buffer: CommandBuffer::new(
                        &device,
                        queue_family_index,
                        CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                        Some(format!("CommandBuffer_{}", frames.len())),
                    )?,
                    render_finished_semaphore: Semaphore::new(
                        &device,
                        Some(format!("RenderFinishedSemaphore_{}", frames.len())),
                    )?,
                    image_available_semaphore: Semaphore::new(
                        &device,
                        Some(format!("ImageAvailableSemaphore_{}", frames.len())),
                    )?,
                    in_flight_fence: Fence::new(
                        &device,
                        Some(format!("InFlightFence_{}", frames.len())),
                    )?,
                })
            }

            swapchain.update(&instance, &device, &surface)?;

            let stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(shader1.handle())
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(shader2.handle())
                    .name(std::ffi::CStr::from_bytes_with_nul(b"main\0").unwrap()),
            ];
            let graphics_pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&vertex_input_state_create_info)
                .input_assembly_state(&input_assembly_state_create_info)
                .viewport_state(&viewport_state_create_info)
                .rasterization_state(&rasterization_state_create_info)
                .multisample_state(&multisample_state_create_info)
                .color_blend_state(&color_blend_state_create_info)
                .dynamic_state(&dynamic_state_create_info)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .subpass(0);
            let graphics_pipeline = device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&graphics_pipeline_create_info),
                    None,
                )
                .map_err(|(_, err)| VulkanGraphicsError::PipelineCreateError(err))?[0];

            Ok(VulkanGraphics {
                vk: VulkanObjects {
                    entry,
                    instance,
                    device,
                    surface,
                    physical_device,
                    debug_messenger,
                    debug_report_callback,
                    pipeline_layout,
                    render_pass,
                    pipeline: graphics_pipeline, // This should be the actual pipeline, not layout
                    swapchain,
                },
                frames,
                current_frame_index: 0,
                shader1: Some(shader1),
                shader2: Some(shader2),
            })
        }
    }

    fn tick(&mut self, renderables: &[Renderable]) -> Result<TickResult, VulkanGraphicsError> {
        let frame_index = match self.vk.swapchain.acquire_next_image(
            &self.vk.device,
            &self.frames[self.current_frame_index].image_available_semaphore,
        ) {
            Ok(image_index) => image_index,
            Err(VulkanGraphicsError::SwapchainSuboptimal) => {
                self.vk
                    .swapchain
                    .update(&self.vk.instance, &self.vk.device, &self.vk.surface)?;
                return Ok(TickResult { drawn_triangles: 0 });
            }
            Err(e) => Err(e)?,
        };

        let frame = &self.frames[frame_index as usize];
        frame.in_flight_fence.wait(&self.vk.device, u64::MAX)?;
        frame.in_flight_fence.reset(&self.vk.device)?;

        let result = self.process_buffer(frame, renderables)?;

        let wait_semaphores = [frame.image_available_semaphore.handle()];
        let signal_semaphores = [frame.render_finished_semaphore.handle()];
        let buffers = [frame.command_buffer.vk_command_buffer];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);
        let queue = unsafe { self.vk.device.get_device_queue(0, 0) };
        unsafe {
            self.vk
                .device
                .queue_submit(queue, &[submit_info], frame.in_flight_fence.vk_fence)
                .map_err(VulkanGraphicsError::UnknownError)?;
        }

        self.vk
            .swapchain
            .queue_present(&self.vk.device, queue, frame_index, None)?;

        self.current_frame_index = (self.current_frame_index + 1) % self.frames.len();

        Ok(result)
    }
}
