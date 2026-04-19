use jay_ash::vk::DebugUtilsMessengerCreateInfoEXT;
use jay_ash::{Entry, Instance, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::ffi::{CStr, c_char};
use std::sync::Arc;
use winit::dpi::PhysicalSize;

use crate::device::VulkanDevice;
use crate::surface::VulkanSurface;
use crate::surface_factory;
use crate::swapchain::VulkanSwapchain;
use crate::vulkan_debug::VulkanDebug;

pub const VALIDATION_LAYERS: &[&CStr] = &[c"VK_LAYER_KHRONOS_validation"];
pub const DEVICE_EXTENSIONS: &[&CStr] = &[
    vk::KHR_SWAPCHAIN_EXTENSION_NAME,
    vk::EXT_EXTENDED_DYNAMIC_STATE_EXTENSION_NAME,
];
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

        Ok(Self {
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
}
