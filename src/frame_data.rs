use std::sync::Arc;

use jay_ash::vk;

use crate::{
    device::Device, graphics_pipeline::Pipeline, renderer::RendererError,
    vertex_buffer::VertexBuffer,
};

pub struct FrameData {
    pub draw_fence: vk::Fence,
    pub present_fence: vk::Fence,
    pub render_finished_semaphore: vk::Semaphore,
    pub present_complete_semaphore: vk::Semaphore,
    pub buffer: vk::CommandBuffer,
    pool: vk::CommandPool,
    device: Arc<Device>,
}

impl FrameData {
    pub fn new(device: Arc<Device>) -> Result<Self, RendererError> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(device.queue.family_index);
        let pool = unsafe { device.logical.create_command_pool(&pool_info, None)? };

        let buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let buffer = unsafe { device.logical.allocate_command_buffers(&buffer_info)? };

        let present_complete_semaphore = unsafe {
            device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };

        let render_finished_semaphore = unsafe {
            device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };

        let present_fence = unsafe {
            device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let draw_fence = unsafe {
            device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        Ok(Self {
            draw_fence,
            present_fence,
            render_finished_semaphore,
            present_complete_semaphore,
            buffer: buffer[0],
            pool,
            device,
        })
    }

    pub fn record_command_buffer(
        &self,
        image: vk::Image,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
        graphics_pipeline: &Pipeline,
        vertex_buffer: &VertexBuffer,
    ) {
        unsafe {
            self.device
                .logical
                .begin_command_buffer(self.buffer, &vk::CommandBufferBeginInfo::default())
                .unwrap();
        };

        self.transition_image_layout(
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::AccessFlags2::empty(),
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        );

        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };
        let attachment_info = vk::RenderingAttachmentInfo::default()
            .image_view(image_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_color);

        let binding = [attachment_info];
        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D::default().extent(extent))
            .layer_count(1)
            .color_attachments(&binding);

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };

        unsafe {
            self.device
                .logical
                .cmd_begin_rendering(self.buffer, &rendering_info);

            self.device.logical.cmd_bind_pipeline(
                self.buffer,
                vk::PipelineBindPoint::GRAPHICS,
                graphics_pipeline.pipeline,
            );

            self.device.logical.cmd_bind_vertex_buffers(
                self.buffer,
                0,
                &[vertex_buffer.buffer],
                &[0],
            );

            self.device
                .logical
                .cmd_set_viewport(self.buffer, 0, &[viewport]);

            self.device
                .logical
                .cmd_set_scissor(self.buffer, 0, &[scissor]);

            self.device
                .logical
                .cmd_draw(self.buffer, vertex_buffer.vertices.len() as u32, 1, 0, 0);

            self.device.logical.cmd_end_rendering(self.buffer);
        }

        self.transition_image_layout(
            image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            vk::AccessFlags2::empty(),
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
        );

        unsafe {
            self.device.logical.end_command_buffer(self.buffer).unwrap();
        }
    }

    fn transition_image_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_access_mask: vk::AccessFlags2,
        dst_access_mask: vk::AccessFlags2,
        src_stage_mask: vk::PipelineStageFlags2,
        dst_stage_mask: vk::PipelineStageFlags2,
    ) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(src_stage_mask)
            .src_access_mask(src_access_mask)
            .dst_stage_mask(dst_stage_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        let binding = [barrier];
        let dependency_info = vk::DependencyInfo::default().image_memory_barriers(&binding);
        unsafe {
            self.device
                .logical
                .cmd_pipeline_barrier2(self.buffer, &dependency_info);
        }
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.logical.destroy_fence(self.draw_fence, None);
            self.device.logical.destroy_fence(self.present_fence, None);
            self.device
                .logical
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device
                .logical
                .destroy_semaphore(self.present_complete_semaphore, None);
            self.device.logical.destroy_command_pool(self.pool, None);
        }
    }
}
