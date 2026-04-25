use std::sync::Arc;

use jay_ash::Instance;
use jay_ash::khr;
use jay_ash::vk;
use jay_ash::vk::SwapchainKHR;
use num::clamp;
use winit::dpi::PhysicalSize;

use crate::device::Device;
use crate::renderer::RendererError;
use crate::surface::Surface;

pub struct VulkanSwapchain {
    pub image_views: Vec<vk::ImageView>,
    pub images: Vec<vk::Image>,
    pub extent: vk::Extent2D,
    pub format: vk::SurfaceFormatKHR,
    pub handle: vk::SwapchainKHR,
    pub loader: khr::swapchain::Device,
    pub device: Arc<Device>,
}

impl VulkanSwapchain {
    pub fn new(
        instance: &Instance,
        device: Arc<Device>,
        surface: &Surface,
        size: PhysicalSize<u32>,
        old_swapchain: Option<SwapchainKHR>,
    ) -> Result<Self, RendererError> {
        let physical_device = device.physical;

        let format = Self::choose_surface_format(physical_device, surface)?;
        let present_mode = Self::choose_present_mode(physical_device, surface)?;

        let capabilities = unsafe {
            surface
                .loader()
                .get_physical_device_surface_capabilities(physical_device, *surface.surface())
        }?;
        let extent = Self::choose_extent(capabilities, size);

        let image_count = Self::choose_image_count(capabilities);

        let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface.surface())
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        if old_swapchain.is_some() {
            swapchain_create_info = swapchain_create_info.old_swapchain(old_swapchain.unwrap());
        }

        let swapchain_device = khr::swapchain::Device::new(instance, &device.logical);
        let swapchain = unsafe { swapchain_device.create_swapchain(&swapchain_create_info, None) }?;
        let images = unsafe { swapchain_device.get_swapchain_images(swapchain) }?;
        let image_views = Self::create_image_views(format, &images, &device.logical)?;

        Ok(Self {
            image_views,
            images,
            extent,
            format,
            handle: swapchain,
            loader: swapchain_device,
            device,
        })
    }

    fn choose_surface_format(
        device: vk::PhysicalDevice,
        surface: &Surface,
    ) -> Result<vk::SurfaceFormatKHR, RendererError> {
        let available_formats = unsafe {
            surface
                .loader()
                .get_physical_device_surface_formats(device, *surface.surface())
        }?;

        let format = available_formats.iter().find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        });

        if format.is_none() {
            eprintln!("Couldn't find optimal surface format");
            return Ok(*available_formats.first().unwrap());
        }

        Ok(*format.unwrap())
    }

    fn choose_present_mode(
        device: vk::PhysicalDevice,
        surface: &Surface,
    ) -> Result<vk::PresentModeKHR, RendererError> {
        let available_present_mode = unsafe {
            surface
                .loader()
                .get_physical_device_surface_present_modes(device, *surface.surface())
        }?;

        let present_mode = available_present_mode
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX);

        if present_mode.is_none() {
            return Ok(vk::PresentModeKHR::FIFO);
        }

        Ok(*present_mode.unwrap())
    }

    fn choose_extent(
        capabilities: vk::SurfaceCapabilitiesKHR,
        size: PhysicalSize<u32>,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            return capabilities.current_extent;
        }

        vk::Extent2D::default()
            .width(clamp(
                size.width,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(clamp(
                size.height,
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
    }

    fn choose_image_count(capabilities: vk::SurfaceCapabilitiesKHR) -> u32 {
        let min_image_count = capabilities.min_image_count.max(3);
        if capabilities.max_image_count > 0 && capabilities.max_image_count < min_image_count {
            return capabilities.max_image_count;
        }

        min_image_count
    }

    fn create_image_views(
        surface_format: vk::SurfaceFormatKHR,
        images: &Vec<vk::Image>,
        device: &jay_ash::Device,
    ) -> Result<Vec<vk::ImageView>, RendererError> {
        let mut image_views = Vec::new();

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(surface_format.format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        for image in images {
            let info = image_view_create_info.image(*image);
            let image_view = unsafe { device.create_image_view(&info, None)? };
            image_views.push(image_view);
        }

        Ok(image_views)
    }
}

impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        unsafe {
            for image_view in &self.image_views {
                self.device.logical.destroy_image_view(*image_view, None);
            }
            self.loader.destroy_swapchain(self.handle, None);
        };
    }
}
