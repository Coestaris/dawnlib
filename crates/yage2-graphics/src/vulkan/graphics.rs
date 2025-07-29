use crate::object::Renderable;
use crate::view::ViewHandle;
use crate::vulkan::device::{get_device_extensions, get_physical_device};
use crate::vulkan::instance::{get_instance_extensions, get_layers, setup_debug};
use crate::vulkan::objects::command_buffer::CommandBuffer;
use crate::vulkan::objects::shader::{Shader, ShaderType};
use crate::vulkan::objects::surface::Surface;
use crate::vulkan::objects::swapchain::Swapchain;
use crate::vulkan::objects::sync::{Fence, Semaphore};
use crate::vulkan::queue::{get_graphics_queue_family_index, get_presentation_queue_family_index};
use crate::vulkan::{GraphicsError, VkObject};
use ash::vk::{CommandBufferResetFlags, CommandPoolCreateFlags};
use ash::{vk, Device, Instance};
use log::{debug, info, warn};
use std::ffi::c_char;
use std::thread::sleep;

#[derive(Clone)]
pub struct GraphicsConfig {}

struct ImageData {
    index: usize,
    queue_submit_fence: Fence,
    swapchain_acquire_semaphore: Semaphore,
    swapchain_release_semaphore: Semaphore,
    command_buffer: CommandBuffer,
}
impl VkObject for ImageData {
    fn name(&self) -> String {
        format!("ImageData_{}", self.index)
    }

    fn destroy(&self, instance: &Instance, device: &Device) -> Result<(), GraphicsError> {
        debug!("Destroying image data: {}", self.name());
        self.command_buffer.destroy(instance, device)?;
        self.swapchain_acquire_semaphore.destroy(instance, device)?;
        self.swapchain_release_semaphore.destroy(instance, device)?;
        self.queue_submit_fence.destroy(instance, device)?;
        Ok(())
    }
}

struct VulkanObjects {
    entry: ash::Entry,
    instance: Instance,
    device: ash::Device,
    debug_messenger: vk::DebugUtilsMessengerEXT,
    debug_report_callback: vk::DebugReportCallbackEXT,
    pipeline: vk::Pipeline, // This should be the actual pipeline, not layout
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,

    presentation_queue: vk::Queue,
    graphics_queue: vk::Queue,

    surface: Surface,
    swapchain: Swapchain,
}

pub struct Graphics {
    vk: VulkanObjects,
    per_image_data: Vec<ImageData>,
    image_index: u32,

    shader1: Option<Shader>,
    shader2: Option<Shader>,
}

#[allow(deprecated)]
impl Drop for Graphics {
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

            for image_data in self.per_image_data.iter_mut() {
                let _ = image_data.destroy(&self.vk.instance, &self.vk.device);
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

pub(crate) struct GraphicsTickResult {
    pub drawn_triangles: u32, // Placeholder for the number of triangles drawn
}

impl Graphics {
    #[inline]
    fn render(
        &self,
        image_index: u32,
        buffer: &CommandBuffer,
        renderables: &[Renderable],
    ) -> Result<GraphicsTickResult, GraphicsError> {
        // Reset the command buffer to release resources and prepare for recording
        buffer.reset(&self.vk.device, CommandBufferResetFlags::empty())?;

        // Begin recording commands into the command buffer
        // This will prepare the command buffer for recording commands
        buffer.begin(
            &self.vk.device,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        )?;

        // Record the commands for rendering
        // This function will handle the actual rendering logic
        let result = self.record_commands(image_index, buffer, renderables)?;
        buffer.end(&self.vk.device)?;

        Ok(result)
    }

    #[inline]
    fn record_commands(
        &self,
        image_index: u32,
        buffer: &CommandBuffer,
        _: &[Renderable],
    ) -> Result<GraphicsTickResult, GraphicsError> {
        let extent = self.vk.swapchain.get_extent();
        let render_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.vk.render_pass)
            .framebuffer(self.vk.swapchain.vk_framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.3, 0.3, 0.0, 1.0],
                },
            }]);

        unsafe {
            self.vk.device.cmd_begin_render_pass(
                buffer.handle(),
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,   // Replace with actual width
            height: extent.height as f32, // Replace with actual height
            min_depth: 0.0,
            max_depth: 1.0,
        };
        unsafe {
            self.vk
                .device
                .cmd_set_viewport(buffer.handle(), 0, std::slice::from_ref(&viewport));
        }

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        unsafe {
            self.vk
                .device
                .cmd_set_scissor(buffer.handle(), 0, std::slice::from_ref(&scissor));
        }

        unsafe {
            self.vk.device.cmd_bind_pipeline(
                buffer.handle(),
                vk::PipelineBindPoint::GRAPHICS,
                self.vk.pipeline, // This should be the actual pipeline, not layout
            );
        }

        // Here you would typically bind vertex buffers, index buffers, etc.

        buffer.draw(
            &self.vk.device,
            3, // Number of vertices to draw, replace with actual count
            1, // Instance count, replace with actual if using instancing
            0, // First vertex
            0, // First instance
        );

        unsafe {
            self.vk.device.cmd_end_render_pass(buffer.handle());
        }

        Ok(GraphicsTickResult {
            drawn_triangles: 1, // Placeholder, actual rendering logic would go here
        })
    }
}

impl Graphics {
    pub(crate) fn open(
        config: GraphicsConfig,
        view_handle: ViewHandle,
    ) -> Result<Self, GraphicsError>
    where
        Self: Sized,
    {
        unsafe {
            debug!("Creating ASH entry");
            let entry = ash::Entry::load().map_err(GraphicsError::EntryCreationError)?;

            debug!("Enumerating Vulkan instance version");
            let vulkan_version = entry
                .try_enumerate_instance_version()
                .map_err(GraphicsError::EnumerateVersionError)?
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
            let instance_extensions = get_instance_extensions(&entry)?;
            let instance_extensions_array = instance_extensions.as_slice();
            let layers = get_layers(&entry)?;
            let layers_array = layers.as_slice();
            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(layers_array)
                .enabled_extension_names(instance_extensions_array);
            let instance = entry
                .create_instance(&create_info, None)
                .map_err(GraphicsError::InstanceCreationError)?;

            let (debug_report_callback, debug_messenger) =
                setup_debug(&entry, &instance, &instance_extensions)?;

            debug!("Creating Vulkan device");
            let physical_device = get_physical_device(&instance)?;
            let device_extensions = get_device_extensions(&instance, physical_device)?;
            let device_extensions_array = device_extensions.as_slice();
            let surface = Surface::new(
                &entry,
                &instance,
                view_handle,
                Some("main_surface".to_string()),
            )?;

            let graphics_queue_family_index =
                get_graphics_queue_family_index(&instance, physical_device)?;
            let presentation_queue_family_index =
                get_presentation_queue_family_index(&instance, physical_device, &surface)?;
            let unique_queues = std::collections::HashSet::from([
                graphics_queue_family_index,
                presentation_queue_family_index,
            ]);
            let priority = [1.0];
            let mut queues_create_infos = vec![];
            for queue_family_index in unique_queues {
                let queue_create_info = vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family_index as u32)
                    .queue_priorities(&priority);
                queues_create_infos.push(queue_create_info);
            }

            let queues = queues_create_infos.as_slice();
            let device_create_info = vk::DeviceCreateInfo::default()
                .enabled_extension_names(device_extensions_array)
                .queue_create_infos(queues);
            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(GraphicsError::CreateDeviceFailed)?;
            let graphics_queue = device.get_device_queue(graphics_queue_family_index as u32, 0);
            let presentation_queue =
                device.get_device_queue(presentation_queue_family_index as u32, 0);

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
                .map_err(GraphicsError::PipelineLayoutCreateError)?;

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

            let dependency = vk::SubpassDependency::default()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);
            let render_pass_create_info = vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color_attachment_description))
                .subpasses(std::slice::from_ref(&subpass_create_info))
                .dependencies(std::slice::from_ref(&dependency));
            let render_pass = device
                .create_render_pass(&render_pass_create_info, None)
                .map_err(GraphicsError::RenderPassCreateError)?;

            let mut swapchain = Swapchain::new(
                &entry,
                &instance,
                &device,
                physical_device,
                render_pass,
                Some("main_swapchain".to_string()),
            )?;

            swapchain.update(&instance, &device, &surface)?;

            let mut per_image_data = vec![];
            for i in 0..swapchain.get_images_count() {
                per_image_data.push(ImageData {
                    index: i,
                    command_buffer: CommandBuffer::new(
                        &device,
                        graphics_queue_family_index,
                        CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                        Some(format!("command_buffer_{}", i)),
                    )?,
                    swapchain_acquire_semaphore: Semaphore::new(
                        &device,
                        Some(format!("swapchain_acquire_semaphore_{}", i)),
                    )?,
                    swapchain_release_semaphore: Semaphore::new(
                        &device,
                        Some(format!("swapchain_release_semaphore_{}", i)),
                    )?,
                    queue_submit_fence: Fence::new(
                        &device,
                        Some(format!("queue_submit_fence_{}", i)),
                    )?,
                })
            }

            info!("Vulkan device created successfully");
            let shader1 = Shader::new_from_file(
                ShaderType::Vertex,
                &device,
                "/home/taris/work/yage2/target/vert",
                Some("triangle_vert".to_string()),
            )?;
            let shader2 = Shader::new_from_file(
                ShaderType::Fragment,
                &device,
                "/home/taris/work/yage2/target/frag",
                Some("triangle_frag".to_string()),
            )?;
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
                .map_err(|(_, err)| GraphicsError::PipelineCreateError(err))?[0];

            Ok(Graphics {
                vk: VulkanObjects {
                    entry,
                    instance,
                    device,
                    surface,
                    debug_messenger,
                    debug_report_callback,
                    pipeline_layout,
                    render_pass,
                    pipeline: graphics_pipeline, // This should be the actual pipeline, not layout
                    swapchain,
                    presentation_queue,
                    graphics_queue,
                },
                per_image_data,
                image_index: 0,
                shader1: Some(shader1),
                shader2: Some(shader2),
            })
        }
    }

    pub(crate) fn tick(
        &mut self,
        renderables: &[Renderable],
    ) -> Result<GraphicsTickResult, GraphicsError> {
        sleep(std::time::Duration::from_millis(16)); // Simulate a frame time of ~60 FPS
        return Ok(GraphicsTickResult { drawn_triangles: 0 });

        // Acquire the next image from the swapchain
        self.image_index = {
            loop {
                // Try to acquire the next image from the swapchain
                // If the swapchain is suboptimal, we will recreate it
                // This will block until the image is ready to be acquired
                // TODO: Implement some kind of timeout or retry logic
                match self.vk.swapchain.acquire_next_image(
                    &self.vk.device,
                    &self.per_image_data[self.image_index as usize].swapchain_acquire_semaphore,
                ) {
                    Ok(image_index) => break image_index,
                    Err(GraphicsError::SwapchainSuboptimal) => {
                        warn!("Swapchain is suboptimal, recreating...");
                        self.vk.swapchain.update(
                            &self.vk.instance,
                            &self.vk.device,
                            &self.vk.surface,
                        )?;

                        // Retry acquiring the next image
                        continue;
                    }

                    Err(e) => Err(e)?,
                }
            }
        };
        // debug!("image: {}", self.image_index);

        let image_data = &self.per_image_data[self.image_index as usize];

        // Wait for the fence to be signaled before proceeding
        image_data
            .queue_submit_fence
            .wait(&self.vk.device, u64::MAX)?;
        image_data.queue_submit_fence.reset(&self.vk.device)?;

        // Process the command buffer for the current frame
        let result = self.render(self.image_index, &image_data.command_buffer, renderables)?;

        // Submit the command buffer to the graphics queue
        // This will block until the command buffer is ready to be submitted
        // Wait for acquire semaphore to be signaled before submitting the command buffer
        let wait_semaphores = [image_data.swapchain_acquire_semaphore.handle()];
        // Signal the swapchain release semaphore after the command buffer has finished executing
        // (will be used to present the image)
        let signal_semaphores = [image_data.swapchain_release_semaphore.handle()];
        // Specify the command buffer to be submitted
        let buffers = [image_data.command_buffer.handle()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.vk
                .device
                .queue_submit(
                    self.vk.graphics_queue,
                    &[submit_info],
                    image_data.queue_submit_fence.vk_fence,
                )
                .map_err(GraphicsError::UnknownError)?;
        }

        // Present the image to the swapchain and handle suboptimal swapchain
        // This will block until the image is ready to be presented
        match self.vk.swapchain.queue_present(
            &self.vk.device,
            self.vk.presentation_queue,
            self.image_index,
            &image_data.swapchain_release_semaphore,
        ) {
            Err(GraphicsError::SwapchainSuboptimal) => {
                warn!("Swapchain is suboptimal, recreating...");
                self.vk
                    .swapchain
                    .update(&self.vk.instance, &self.vk.device, &self.vk.surface)?;
            }
            Err(e) => Err(e)?,
            _ => {}
        }

        Ok(result)
    }
}
