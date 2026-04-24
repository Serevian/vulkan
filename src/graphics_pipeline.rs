use std::{io::Cursor, sync::Arc};

use jay_ash::vk;

use crate::{device::VulkanDevice, renderer::RendererError, swapchain::VulkanSwapchain};

pub struct GraphicsPipeline {
    pub pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    device: Arc<VulkanDevice>,
}

impl GraphicsPipeline {
    pub fn new(
        device: Arc<VulkanDevice>,
        swapchain: &VulkanSwapchain,
    ) -> Result<Self, RendererError> {
        let (pipeline_shader_info, shader_module) = Self::create_shader_modules(&device)?;

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);
        let binding = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&binding);

        let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe {
            device
                .logical
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let binding = [swapchain.format.format];
        let mut pipeline_rendering_info =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&binding);
        let graphics_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&pipeline_shader_info)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .push_next(&mut pipeline_rendering_info);
        let graphics_pipeline = unsafe {
            device
                .logical
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphics_pipeline_info],
                    None,
                )
                .unwrap()
        };

        unsafe {
            device
                .logical
                .destroy_shader_module(shader_module, None);
        }

        Ok(Self {
            pipeline: graphics_pipeline[0],
            pipeline_layout,
            device,
        })
    }

    fn create_shader_modules(
        device: &Arc<VulkanDevice>,
    ) -> Result<(Vec<vk::PipelineShaderStageCreateInfo<'_>>, vk::ShaderModule), RendererError> {
        let shader_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/shader.spv"));
        let shader_code = jay_ash::util::read_spv(&mut Cursor::new(shader_bytes)).unwrap();
        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);
        let shader_module = unsafe {
            device
                .logical
                .create_shader_module(&shader_module_info, None)?
        };

        let mut shader_stages_info = Vec::new();

        let vertex_shader_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_module)
            .name(c"vertMain");
        shader_stages_info.push(vertex_shader_info);

        let fragment_shader_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_module)
            .name(c"fragMain");
        shader_stages_info.push(fragment_shader_info);

        Ok((shader_stages_info, shader_module))
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical
                .destroy_pipeline(self.pipeline, None);

            self.device
                .logical
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
