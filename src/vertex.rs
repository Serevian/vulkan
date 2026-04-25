use glam::{Vec2, Vec3};
use jay_ash::vk;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vec2,
    pub color: Vec3,
}

impl Vertex {
    pub fn new(pos: Vec2, color: Vec3) -> Self {
        Self { pos, color }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        let stride = std::mem::size_of::<Self>() as u32;
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(stride)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(std::mem::offset_of!(Self, pos) as u32),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(std::mem::offset_of!(Self, color) as u32),
        ]
    }
}
