use std::os::raw::c_char;

use jay_ash::{
    Entry, Instance,
    ext::metal_surface,
    khr::{android_surface, surface, wayland_surface, win32_surface, xcb_surface, xlib_surface},
    prelude::VkResult,
    vk,
};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[derive(Clone)]
enum SurfaceExtension {
    Windows(win32_surface::Instance),
    Wayland(
        raw_window_handle::WaylandDisplayHandle,
        wayland_surface::Instance,
    ),
    Xlib(raw_window_handle::XlibDisplayHandle, xlib_surface::Instance),
    Xcb(raw_window_handle::XcbDisplayHandle, xcb_surface::Instance),
    Android(android_surface::Instance),
    #[cfg(target_os = "macos")]
    AppKit(metal_surface::Instance),
    #[cfg(target_os = "ios")]
    UiKit(metal_surface::Instance),
}

/// Holder for a loaded platform-specific Vulkan `Surface` extension, used to create surfaces.
///
/// Also stores the platform-specific [`raw_window_handle::RawDisplayHandle`] variant if necessary
/// to create surfaces, identifying the selected display server handle that was used to load the
/// relevant extension when creating a [`SurfaceFactory`].
#[derive(Clone)]
pub struct SurfaceFactory(SurfaceExtension);

impl SurfaceFactory {
    /// Load the relevant surface extension for a given [`RawDisplayHandle`].
    ///
    /// `instance` must have been created with platform specific surface extensions enabled, acquired
    /// through [`enumerate_required_extensions()`].
    pub fn new(
        entry: &Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
    ) -> VkResult<Self> {
        Ok(Self(match display_handle {
            RawDisplayHandle::Windows(_) => {
                SurfaceExtension::Windows(win32_surface::Instance::new(entry, instance))
            }

            RawDisplayHandle::Wayland(display) => {
                SurfaceExtension::Wayland(display, wayland_surface::Instance::new(entry, instance))
            }

            RawDisplayHandle::Xlib(display) => {
                SurfaceExtension::Xlib(display, xlib_surface::Instance::new(entry, instance))
            }

            RawDisplayHandle::Xcb(display) => {
                SurfaceExtension::Xcb(display, xcb_surface::Instance::new(entry, instance))
            }

            RawDisplayHandle::Android(_) => {
                SurfaceExtension::Android(android_surface::Instance::new(entry, instance))
            }

            #[cfg(target_os = "macos")]
            RawDisplayHandle::AppKit(_) => {
                SurfaceExtension::AppKit(metal_surface::Instance::new(entry, instance))
            }

            #[cfg(target_os = "ios")]
            RawDisplayHandle::UiKit(_) => {
                SurfaceExtension::UiKit(metal_surface::Instance::new(entry, instance))
            }

            _ => return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
        }))
    }

    pub unsafe fn create_surface(
        &self,
        window_handle: RawWindowHandle,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> VkResult<vk::SurfaceKHR> {
        match (&self.0, window_handle) {
            (SurfaceExtension::Windows(surface_fn), RawWindowHandle::Win32(window)) => {
                let surface_desc = vk::Win32SurfaceCreateInfoKHR::default()
                    .hwnd(window.hwnd.get())
                    .hinstance(
                        window
                            .hinstance
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .get(),
                    );
                surface_fn.create_win32_surface(&surface_desc, allocation_callbacks)
            }

            (SurfaceExtension::Wayland(display, surface_fn), RawWindowHandle::Wayland(window)) => {
                let surface_desc = vk::WaylandSurfaceCreateInfoKHR::default()
                    .display(display.display.as_ptr())
                    .surface(window.surface.as_ptr());
                surface_fn.create_wayland_surface(&surface_desc, allocation_callbacks)
            }

            (SurfaceExtension::Xlib(display, surface_fn), RawWindowHandle::Xlib(window)) => {
                let surface_desc = vk::XlibSurfaceCreateInfoKHR::default()
                    .dpy(
                        display
                            .display
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .as_ptr(),
                    )
                    .window(window.window);
                surface_fn.create_xlib_surface(&surface_desc, allocation_callbacks)
            }

            (SurfaceExtension::Xcb(display, surface_fn), RawWindowHandle::Xcb(window)) => {
                let surface_desc = vk::XcbSurfaceCreateInfoKHR::default()
                    .connection(
                        display
                            .connection
                            .ok_or(vk::Result::ERROR_INITIALIZATION_FAILED)?
                            .as_ptr(),
                    )
                    .window(window.window.get());
                surface_fn.create_xcb_surface(&surface_desc, allocation_callbacks)
            }

            (SurfaceExtension::Android(surface_fn), RawWindowHandle::AndroidNdk(window)) => {
                let surface_desc = vk::AndroidSurfaceCreateInfoKHR::default()
                    .window(window.a_native_window.as_ptr());
                surface_fn.create_android_surface(&surface_desc, allocation_callbacks)
            }

            #[cfg(target_os = "macos")]
            (SurfaceExtension::AppKit(surface_fn), RawWindowHandle::AppKit(window)) => {
                use raw_window_metal::{Layer, appkit};

                let layer = match appkit::metal_layer_from_handle(window) {
                    Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                };

                let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
            }

            #[cfg(target_os = "ios")]
            (SurfaceExtension::UiKit(surface_fn), RawWindowHandle::UiKit(window)) => {
                use raw_window_metal::{Layer, uikit};

                let layer = match uikit::metal_layer_from_handle(window) {
                    Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                };

                let surface_desc = vk::MetalSurfaceCreateInfoEXT::default().layer(&*layer);
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)
            }

            _ => Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
        }
    }
}
