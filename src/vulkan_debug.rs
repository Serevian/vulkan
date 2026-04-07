use jay_ash::Entry;
use jay_ash::{Instance, ext, vk};
use std::ffi::CStr;
use std::ffi::c_void;

use crate::renderer::{RendererError, VALIDATION_LAYERS};

#[cfg(debug_assertions)]
pub struct VulkanDebug {
    debug_utils: ext::debug_utils::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

#[cfg(debug_assertions)]
impl VulkanDebug {
    pub fn new(
        entry: &Entry,
        instance: &Instance,
        debug_create_info: vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> Result<Self, RendererError> {
        let debug_utils = ext::debug_utils::Instance::new(entry, &instance);
        let debug_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None)? };

        Ok(Self {
            debug_utils,
            debug_messenger,
        })
    }

    pub fn check_validation_layers(entry: &Entry) -> Result<(), RendererError> {
        let layer_properties = unsafe { entry.enumerate_instance_layer_properties()? };

        let missing: Vec<String> = VALIDATION_LAYERS
            .iter()
            .filter(|&&needed_layer| {
                !layer_properties
                    .iter()
                    .any(|props| props.layer_name_as_c_str() == Ok(needed_layer))
            })
            .map(|&layer| layer.to_string_lossy().into_owned())
            .collect();

        if !missing.is_empty() {
            return Err(RendererError::UnsupportedLayers(missing));
        }

        Ok(())
    }

    #[cfg(debug_assertions)]
    pub fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::vulkan_debug_callback))
    }

    #[cfg(debug_assertions)]
    unsafe extern "system" fn vulkan_debug_callback(
        severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        m_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
        _user_data: *mut c_void,
    ) -> vk::Bool32 {
        let data = unsafe { *p_data };
        let message = unsafe { CStr::from_ptr(data.p_message) }.to_string_lossy();
        println!("[{severity:?}] [{m_type:?}] {message}");
        vk::FALSE
    }
}

impl Drop for VulkanDebug {
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_messenger, None);
        }
    }
}
