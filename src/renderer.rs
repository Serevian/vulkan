use jay_ash::vk::DebugUtilsMessengerCreateInfoEXT;
use jay_ash::{Entry, Instance, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::{CStr, c_char};
use std::sync::Arc;
use winit::dpi::PhysicalSize;

use crate::command_context::CommandContext;
use crate::device::VulkanDevice;
use crate::graphics_pipeline::GraphicsPipeline;
use crate::surface::VulkanSurface;
use crate::surface_factory;
use crate::swapchain::VulkanSwapchain;
use crate::vulkan_debug::VulkanDebug;

pub const VALIDATION_LAYERS: &[&CStr] = &[c"VK_LAYER_KHRONOS_validation"];
pub const DEVICE_EXTENSIONS: &[&CStr] = &[vk::KHR_SWAPCHAIN_EXTENSION_NAME];
pub const DEBUG_EXTENSIONS: &[&CStr] = &[vk::EXT_DEBUG_UTILS_EXTENSION_NAME];

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
    draw_fence: vk::Fence,
    render_finished: vk::Semaphore,
    present_complete: vk::Semaphore,
    command_context: CommandContext,
    graphics_pipeline: GraphicsPipeline,
    swapchain: VulkanSwapchain,
    device: Arc<VulkanDevice>,
    surface: VulkanSurface,
    debug: VulkanDebug,
    instance: Instance,
    entry: Entry,
}

impl Renderer {
    pub fn new(
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
        size: PhysicalSize<u32>,
    ) -> Result<Self, RendererError> {
        let entry = Entry::linked();

        let (instance, debug_info) = Self::new_instance(&entry, raw_display_handle)?;

        let debug = VulkanDebug::new(&entry, &instance, debug_info)?;

        let surface = VulkanSurface::new(&entry, &instance, raw_display_handle, raw_window_handle)?;

        let device = Arc::new(VulkanDevice::new(&instance, &surface)?);

        let swapchain = VulkanSwapchain::new(&instance, device.clone(), &surface, size)?;

        let graphics_pipeline = GraphicsPipeline::new(device.clone(), &swapchain)?;

        let command_context = CommandContext::new(device.clone())?;

        let present_complete = unsafe {
            device
                .logical_device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };

        let render_finished = unsafe {
            device
                .logical_device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };

        let draw_fence = unsafe {
            device.logical_device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        Ok(Self {
            draw_fence,
            render_finished,
            present_complete,
            command_context,
            graphics_pipeline,
            swapchain,
            device,
            surface,
            debug,
            instance,
            entry,
        })
    }

    fn new_instance(
        entry: &Entry,
        raw_display_handle: RawDisplayHandle,
    ) -> Result<(Instance, DebugUtilsMessengerCreateInfoEXT<'_>), RendererError> {
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Hello Triangle")
            .engine_name(c"Vulkan Engine")
            .api_version(vk::API_VERSION_1_4);

        let extensions = Self::check_instance_extensions(raw_display_handle)?;

        #[cfg(debug_assertions)]
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

    fn check_instance_extensions(
        raw_display_handle: RawDisplayHandle,
    ) -> Result<Vec<*const c_char>, RendererError> {
        // Already checks if we have a supported surface
        let window_extensions = surface_factory::enumerate_required_extensions(raw_display_handle)
            .map_err(|_| vk::Result::ERROR_EXTENSION_NOT_PRESENT)?;

        let mut extensions = window_extensions.to_vec();

        if cfg!(debug_assertions) {
            extensions.extend(DEBUG_EXTENSIONS.iter().map(|ext| ext.as_ptr()));
        }

        Ok(extensions)
    }

    pub fn draw(&self) -> Result<(), RendererError> {
        unsafe {
            self.device
                .logical_device
                .wait_for_fences(&[self.draw_fence], true, u64::MAX)?;

            self.device
                .logical_device
                .reset_fences(&[self.draw_fence])?;

            let (image_index, suboptimal) = self.swapchain.loader.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.present_complete,
                vk::Fence::null(),
            )?;

            let image = self.swapchain.images[image_index as usize];
            let image_view = self.swapchain.image_views[image_index as usize];
            self.command_context.record_command_buffer(
                image,
                image_view,
                self.swapchain.extent,
                &self.graphics_pipeline,
            );

            let wait_destination_stage_mask = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;

            let present_temp = [self.present_complete];
            let wait_temp = [wait_destination_stage_mask];
            let command_temp = [self.command_context.buffer];
            let render_temp = [self.render_finished];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&present_temp)
                .wait_dst_stage_mask(&wait_temp)
                .command_buffers(&command_temp)
                .signal_semaphores(&render_temp);

            let queue = self
                .device
                .logical_device
                .get_device_queue(self.device.queue.family_index, self.device.queue.index);

            self.device
                .logical_device
                .queue_submit(queue, &[submit_info], self.draw_fence)?;

            let swapchain_temp = [self.swapchain.swapchain];
            let index_temp = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&render_temp)
                .swapchains(&swapchain_temp)
                .image_indices(&index_temp);

            let result = self.swapchain.loader.queue_present(queue, &present_info);
        };

        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.logical_device.device_wait_idle();

            self.device
                .logical_device
                .destroy_fence(self.draw_fence, None);
            self.device
                .logical_device
                .destroy_semaphore(self.render_finished, None);
            self.device
                .logical_device
                .destroy_semaphore(self.present_complete, None);
        }
    }
}
