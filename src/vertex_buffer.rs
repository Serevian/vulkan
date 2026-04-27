use std::sync::Arc;

use jay_ash::vk;

use crate::{device::Device, renderer::RendererError, vertex::Vertex};

pub struct VertexBuffer {
    memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,
    pub vertices: Vec<Vertex>,
    device: Arc<Device>,
}

impl VertexBuffer {
    pub fn new(device: Arc<Device>, vertices: &[Vertex]) -> Result<Self, RendererError> {
        let size = std::mem::size_of_val(vertices) as u64;

        let (staging_buffer, staging_memory) = Self::new_buffer(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &device,
        )?;

        unsafe {
            let data =
                device
                    .logical
                    .map_memory(staging_memory, 0, size, vk::MemoryMapFlags::empty())?;

            let bytes = bytemuck::cast_slice(vertices);

            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());

            device.logical.unmap_memory(staging_memory);
        }

        let (vertex_buffer, vertex_memory) = Self::new_buffer(
            size,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &device,
        )?;

        Self::copy_buffer(staging_buffer, vertex_buffer, size, &device)?;

        unsafe {
            device.logical.free_memory(staging_memory, None);
            device.logical.destroy_buffer(staging_buffer, None);
        }

        Ok(Self {
            memory: vertex_memory,
            buffer: vertex_buffer,
            vertices: vertices.to_vec(),
            device,
        })
    }

    fn new_buffer(
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        device: &Arc<Device>,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), RendererError> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.logical.create_buffer(&buffer_info, None)? };

        let memory_requirements = unsafe { device.logical.get_buffer_memory_requirements(buffer) };

        let memory_type_filter = memory_requirements.memory_type_bits;

        let memory_type_index = device
            .memory_properties
            .memory_types_as_slice()
            .iter()
            .enumerate()
            .find(|(i, memory_type)| {
                // Weird as fuck bitwise match. Perhaps it could be better done, idk.
                let filter_match = (memory_type_filter & (1 << i)) != 0;
                let flags_match = memory_type.property_flags.contains(properties);

                filter_match && flags_match
            })
            .map(|(i, _)| i as u32)
            .expect("No suitable memory type found");

        let memory_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.logical.allocate_memory(&memory_info, None)? };

        unsafe {
            device.logical.bind_buffer_memory(buffer, memory, 0)?;
        }

        Ok((buffer, memory))
    }

    fn copy_buffer(
        source: vk::Buffer,
        destination: vk::Buffer,
        size: vk::DeviceSize,
        device: &Arc<Device>,
    ) -> Result<(), RendererError> {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(device.queue.family_index);

        let pool = unsafe { device.logical.create_command_pool(&pool_info, None)? };

        let buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { device.logical.allocate_command_buffers(&buffer_info)?[0] };

        unsafe {
            device.logical.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            device.logical.cmd_copy_buffer(
                command_buffer,
                source,
                destination,
                &[vk::BufferCopy::default()
                    .src_offset(0)
                    .dst_offset(0)
                    .size(size)],
            );

            device.logical.end_command_buffer(command_buffer)?;

            device.logical.queue_submit(
                device.queue.handle,
                &[vk::SubmitInfo::default().command_buffers(&[command_buffer])],
                vk::Fence::null(),
            )?;

            device.logical.queue_wait_idle(device.queue.handle)?;

            device.logical.destroy_command_pool(pool, None);
        }

        Ok(())
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.logical.free_memory(self.memory, None);
            self.device.logical.destroy_buffer(self.buffer, None);
        }
    }
}
