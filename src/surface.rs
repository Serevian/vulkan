use jay_ash::{Entry, Instance, khr, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{renderer::RendererError, surface_factory::SurfaceFactory};

pub struct VulkanSurface {
    surface: vk::SurfaceKHR,
    loader: khr::surface::Instance,
}

impl VulkanSurface {
    pub fn new(
        entry: &Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self, RendererError> {
        let surface = unsafe {
            SurfaceFactory::new(entry, &instance, display_handle)?
                .create_surface(window_handle, None)?
        };

        let surface_loader = khr::surface::Instance::new(entry, &instance);

        Ok(Self {
            surface,
            loader: surface_loader,
        })
    }

    pub fn loader(&self) -> &khr::surface::Instance {
        &self.loader
    }

    pub fn surface(&self) -> &vk::SurfaceKHR {
        &self.surface
    }
}

impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
