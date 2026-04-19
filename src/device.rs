use std::ffi::c_char;

use jay_ash::{Instance, vk};

use crate::{
    renderer::{DEVICE_EXTENSIONS, RendererError},
    surface::VulkanSurface,
};

pub struct Queue {
    handle: vk::Queue,
    family_index: u32,
    index: u32,
}

impl Queue {
    pub const fn new(handle: vk::Queue, family_index: u32, index: u32) -> Self {
        Self {
            handle,
            family_index,
            index,
        }
    }
}

pub struct VulkanDevice {
    queue: Queue,
    pub logical_device: jay_ash::Device,
    pub physical_device: vk::PhysicalDevice,
}

impl VulkanDevice {
    pub fn new(instance: &Instance, surface: &VulkanSurface) -> Result<Self, RendererError> {
        let physical_device = Self::new_physical_device(instance)?;
        let (logical_device, graphics_queue) =
            Self::new_logical_device(physical_device, instance, surface)?;

        Ok(Self {
            queue: graphics_queue,
            logical_device,
            physical_device,
        })
    }

    fn new_physical_device(instance: &Instance) -> Result<vk::PhysicalDevice, RendererError> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let physical_device = physical_devices
            .into_iter()
            .find_map(|device| {
                Self::is_device_suitable(device, instance)
                    .ok()
                    .and_then(|suitable| if suitable { Some(device) } else { None })
            })
            .ok_or(RendererError::GPUNotFound)?;

        Ok(physical_device)
    }

    fn new_logical_device(
        physical_device: vk::PhysicalDevice,
        instance: &Instance,
        surface: &VulkanSurface,
    ) -> Result<(jay_ash::Device, Queue), RendererError> {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let queue_index = unsafe {
            queue_family_properties
                .iter()
                .enumerate()
                .position(|(index, qfp)| {
                    qfp.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && surface
                            .loader()
                            .get_physical_device_surface_support(
                                physical_device,
                                index as u32,
                                *surface.surface(),
                            )
                            .unwrap_or(false)
                })
        };
        if queue_index.is_none() {
            return Err(RendererError::QueueNotFound);
        }

        let device_queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_index.unwrap() as u32)
            .queue_priorities(&[0.5]);

        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default();
        let mut extended_dynamic_state_features =
            vk::PhysicalDeviceExtendedDynamicStateFeaturesEXT::default();

        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vulkan13_features)
            .push_next(&mut extended_dynamic_state_features);

        let extension_ptrs: Vec<*const c_char> =
            DEVICE_EXTENSIONS.iter().map(|s| s.as_ptr()).collect();

        let binding = [device_queue_create_info];
        let device_create_info = vk::DeviceCreateInfo::default()
            .push_next(&mut features2)
            .queue_create_infos(&binding)
            .enabled_extension_names(&extension_ptrs);

        let logical_device =
            unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let queue_handle =
            unsafe { logical_device.get_device_queue(queue_index.unwrap() as u32, 0) };

        let queue = Queue::new(queue_handle, queue_index.unwrap() as u32, 0);

        Ok((logical_device, queue))
    }

    fn is_device_suitable(
        device: vk::PhysicalDevice,
        instance: &Instance,
    ) -> Result<bool, RendererError> {
        let properties = unsafe { instance.get_physical_device_properties(device) };

        let supports_vulkan14 = properties.api_version >= vk::API_VERSION_1_4;

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };
        let supports_graphics = queue_families
            .iter()
            .any(|queue_family| queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        let available_extensions =
            unsafe { instance.enumerate_device_extension_properties(device)? };

        let supports_extensions = DEVICE_EXTENSIONS.iter().all(|ext| {
            available_extensions
                .iter()
                .any(|av_ext| av_ext.extension_name_as_c_str().unwrap() == ext)
        });

        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default();
        let mut extended_dynamic_state_features =
            vk::PhysicalDeviceExtendedDynamicStateFeaturesEXT::default();

        let mut features2 = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vulkan13_features)
            .push_next(&mut extended_dynamic_state_features);

        unsafe {
            instance.get_physical_device_features2(device, &mut features2);
        }

        let supports_required_features = vulkan13_features.dynamic_rendering == vk::TRUE
            && extended_dynamic_state_features.extended_dynamic_state == vk::TRUE;

        Ok(supports_vulkan14
            && supports_graphics
            && supports_extensions
            && supports_required_features)
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_device(None);
        }
    }
}
