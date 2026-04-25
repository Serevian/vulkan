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
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.logical.create_buffer(&buffer_info, None)? };

        let memory_requirements = unsafe { device.logical.get_buffer_memory_requirements(buffer) };

        let memory_type_filter = memory_requirements.memory_type_bits;
        let required_memory_flags =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;

        let memory_type_index = device
            .memory_properties
            .memory_types_as_slice()
            .iter()
            .enumerate()
            .find(|(i, memory_type)| {
                // Weird as fuck bitwise match. Perhaps it could be better done, idk.
                let filter_match = (memory_type_filter & (1 << i)) != 0;
                let flags_match = memory_type.property_flags.contains(required_memory_flags);

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
            let data = device.logical.map_memory(
                memory,
                0,
                buffer_info.size,
                vk::MemoryMapFlags::empty(),
            )?;

            let bytes = bytemuck::cast_slice(vertices);

            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());

            device.logical.unmap_memory(memory);
        }

        Ok(Self {
            memory,
            buffer,
            vertices: vertices.to_vec(),
            device,
        })
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
