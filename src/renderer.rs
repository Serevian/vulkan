use jay_ash::vk::DebugUtilsMessengerCreateInfoEXT;
use jay_ash::{Entry, Instance, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::{CStr, c_char};
use std::sync::Arc;
use winit::dpi::PhysicalSize;

use crate::device::Device;
use crate::frame_data::FrameData;
use crate::graphics_pipeline::Pipeline;
use crate::surface::Surface;
use crate::surface_factory;
use crate::swapchain::VulkanSwapchain;
use crate::vertex::Vertex;
use crate::vertex_buffer::VertexBuffer;
use crate::vulkan_debug::VulkanDebug;

pub const VALIDATION_LAYERS: &[&CStr] = &[c"VK_LAYER_KHRONOS_validation"];
pub const INSTANCE_EXTENSIONS: &[&CStr] = &[
    vk::KHR_GET_SURFACE_CAPABILITIES_2_EXTENSION_NAME,
    vk::KHR_SURFACE_MAINTENANCE_1_EXTENSION_NAME,
];
pub const DEVICE_EXTENSIONS: &[&CStr] = &[
    vk::KHR_SWAPCHAIN_EXTENSION_NAME,
    vk::KHR_SWAPCHAIN_MAINTENANCE_1_EXTENSION_NAME,
];
pub const DEBUG_EXTENSIONS: &[&CStr] = &[vk::EXT_DEBUG_UTILS_EXTENSION_NAME];
pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("Required layers not supported: {0:?}")]
    UnsupportedLayers(Vec<String>),
    #[error("Suitable GPU not found")]
    GPUNotFound,
    #[error("Suitable queue not found")]
    QueueNotFound,
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct Renderer {
    current_frame_index: usize,
    frame_data: Vec<FrameData>,
    vertex_buffer: VertexBuffer,
    graphics_pipeline: Pipeline,
    swapchain: VulkanSwapchain,
    device: Arc<Device>,
    surface: Surface,
    debug: Option<VulkanDebug>,
    instance: Instance,
    entry: Entry,
}

impl Renderer {
    pub fn new(
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
        size: PhysicalSize<u32>,
    ) -> Result<Self, RendererError> {
        let vertices = [
            Vertex::new(
                glam::Vec2 { x: 0.0, y: -0.5 },
                glam::Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ),
            Vertex::new(
                glam::Vec2 { x: 0.5, y: 0.5 },
                glam::Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ),
            Vertex::new(
                glam::Vec2 { x: -0.5, y: 0.5 },
                glam::Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
            ),
        ];

        let entry = Entry::linked();

        #[cfg(debug_assertions)]
        let (instance, debug_info) = Self::new_debug_instance(&entry, raw_display_handle)?;
        #[cfg(debug_assertions)]
        let debug = VulkanDebug::new(&entry, &instance, debug_info)?;
        #[cfg(debug_assertions)]
        let debug = Some(debug);
        #[cfg(not(debug_assertions))]
        let instance = Self::new_instance(&entry, raw_display_handle)?;
        #[cfg(not(debug_assertions))]
        let debug = None;

        let surface = Surface::new(&entry, &instance, raw_display_handle, raw_window_handle)?;

        let device = Arc::new(Device::new(&instance, &surface)?);

        let swapchain = VulkanSwapchain::new(&instance, device.clone(), &surface, size, None)?;

        let graphics_pipeline = Pipeline::new(device.clone(), &swapchain)?;

        let vertex_buffer = VertexBuffer::new(device.clone(), &vertices)?;

        let mut frame_data = Vec::new();
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let frame = FrameData::new(device.clone())?;
            frame_data.push(frame);
        }

        Ok(Self {
            current_frame_index: 0,
            frame_data,
            vertex_buffer,
            graphics_pipeline,
            swapchain,
            device,
            surface,
            debug,
            instance,
            entry,
        })
    }

    #[cfg(debug_assertions)]
    fn new_debug_instance(
        entry: &Entry,
        raw_display_handle: RawDisplayHandle,
    ) -> Result<(Instance, DebugUtilsMessengerCreateInfoEXT<'_>), RendererError> {
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Hello Triangle")
            .engine_name(c"Vulkan Engine")
            .api_version(vk::API_VERSION_1_4);

        let extensions = Self::check_instance_extensions(raw_display_handle)?;

        let mut debug_create_info = VulkanDebug::debug_messenger_create_info();

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        let layer_ptrs: Vec<*const c_char> = VALIDATION_LAYERS
            .iter()
            .map(|layer| layer.as_ptr())
            .collect();

        if cfg!(debug_assertions) {
            VulkanDebug::check_validation_layers(entry)?;
            create_info = create_info
                .enabled_layer_names(&layer_ptrs)
                .push_next(&mut debug_create_info);
        }

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok((instance, debug_create_info))
    }

    fn new_instance(
        entry: &Entry,
        raw_display_handle: RawDisplayHandle,
    ) -> Result<Instance, RendererError> {
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Hello Triangle")
            .engine_name(c"Vulkan Engine")
            .api_version(vk::API_VERSION_1_4);

        let extensions = Self::check_instance_extensions(raw_display_handle)?;

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(instance)
    }

    fn check_instance_extensions(
        raw_display_handle: RawDisplayHandle,
    ) -> Result<Vec<*const c_char>, RendererError> {
        // Already checks if we have a supported surface
        let window_extensions = surface_factory::enumerate_required_extensions(raw_display_handle)
            .map_err(|_| vk::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut extensions = window_extensions.to_vec();
        extensions.extend(INSTANCE_EXTENSIONS.iter().map(|ext| ext.as_ptr()));

        if cfg!(debug_assertions) {
            extensions.extend(DEBUG_EXTENSIONS.iter().map(|ext| ext.as_ptr()));
        }

        Ok(extensions)
    }

    pub fn draw(&mut self) -> Result<(), RendererError> {
        let frame = &self.frame_data[self.current_frame_index];

        unsafe {
            // Wait until the GPU is done with this frame's resources (rendering done) and swapchain image (presenting done)
            self.device.logical.wait_for_fences(
                &[frame.draw_fence, frame.present_fence],
                true,
                u64::MAX,
            )?;

            self.device
                .logical
                .reset_fences(&[frame.draw_fence, frame.present_fence])?;

            // No need for the CPU to wait here because we already know the image is free
            // Thanks to the previous present fence
            let (image_index, _) = self.swapchain.loader.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                frame.present_complete_semaphore,
                vk::Fence::null(),
            )?;

            let image = self.swapchain.images[image_index as usize];
            let image_view = self.swapchain.image_views[image_index as usize];
            frame.record_command_buffer(
                image,
                image_view,
                self.swapchain.extent,
                &self.graphics_pipeline,
                &self.vertex_buffer,
            );

            // Fuck lifetimes
            let wait_destination_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let present_temp = [frame.present_complete_semaphore];
            let buffer_temp = [frame.buffer];
            let render_temp = [frame.render_finished_semaphore];

            // We wait until image is safe to write
            // Then signal that rendering can proceed
            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&present_temp)
                .wait_dst_stage_mask(&wait_destination_stage_mask)
                .command_buffers(&buffer_temp)
                .signal_semaphores(&render_temp);

            let queue = self
                .device
                .logical
                .get_device_queue(self.device.queue.family_index, self.device.queue.index);

            // TODO: Use features from VK_KHR_swapchain_maintenance1, they can help here if the queue submit or present give any errors
            self.device
                .logical
                .queue_submit(queue, &[submit_info], frame.draw_fence)?;

            let present_fence_temp = [frame.present_fence];
            let mut present_fence_info =
                vk::SwapchainPresentFenceInfoKHR::default().fences(&present_fence_temp);

            let swapchain_temp = [self.swapchain.handle];
            let index_temp = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .push_next(&mut present_fence_info)
                .wait_semaphores(&render_temp)
                .swapchains(&swapchain_temp)
                .image_indices(&index_temp);

            let _ = self.swapchain.loader.queue_present(queue, &present_info);
        };

        self.current_frame_index = (self.current_frame_index + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn recreate_swapchain(&mut self, new_size: PhysicalSize<u32>) -> Result<(), RendererError> {
        unsafe { self.device.logical.device_wait_idle()? }

        let new_swapchain = VulkanSwapchain::new(
            &self.instance,
            self.device.clone(),
            &self.surface,
            new_size,
            Some(self.swapchain.handle),
        )?;

        let new_graphics_pipeline = Pipeline::new(self.device.clone(), &new_swapchain)?;

        self.swapchain = new_swapchain;
        self.graphics_pipeline = new_graphics_pipeline;

        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.logical.device_wait_idle();
        }
    }
}
