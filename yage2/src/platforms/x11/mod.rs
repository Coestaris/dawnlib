use crate::engine::application::{Application, ApplicationConfig, ApplicationError};
use crate::engine::graphics::Graphics;
use crate::engine::vulkan::{VulkanGraphics, VulkanGraphicsError, VulkanGraphicsInitArgs};
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use ash::vk;
use log::{debug, info};
use std::ffi::c_char;
use std::ptr::addr_of_mut;
use std::sync::Arc;
use x11::xlib::{
    XAutoRepeatOff, XClearWindow, XDefaultScreen, XMapRaised, XOpenDisplay, XStoreName, XSync,
};

#[derive(Debug)]
#[allow(dead_code)]
pub enum X11Error {
    OpenDisplayError,
    CreateWindowError,
    GraphicsCreateError(VulkanGraphicsError),
    VulkanCreateSurfaceError(vk::Result),
    VulkanUpdateSurfaceError(VulkanGraphicsError),
}

pub struct X11Window {
    display: *mut x11::xlib::Display,
    window: x11::xlib::Window,
    graphics: VulkanGraphics,
}

pub struct X11Application {
    window_factory: X11WindowFactory,
}

impl Application<X11Error, VulkanGraphics, X11Window> for X11Application {
    fn new(config: ApplicationConfig) -> Result<X11Application, ApplicationError<X11Error>>
    where
        Self: Sized,
    {
        info!("Creating X11 application with config: {:?}", config);
        let window_factory =
            X11WindowFactory::new(config.window_config).map_err(ApplicationError::InitError)?;
        Ok(X11Application { window_factory })
    }

    fn get_window_factory(&self) -> Arc<dyn WindowFactory<X11Window, X11Error, VulkanGraphics> + Send + Sync> {
        Arc::new(self.window_factory.clone())
    }
}

#[derive(Clone, Debug)]
struct X11WindowFactory {
    config: WindowConfig,
}

impl WindowFactory<X11Window, X11Error, VulkanGraphics> for X11WindowFactory {
    fn new(config: WindowConfig) -> Result<Self, X11Error>
    where
        Self: Sized,
    {
        info!("Creating X11 window factory with config: {:?}", config);
        Ok(X11WindowFactory { config })
    }

    fn create_window(&self) -> Result<X11Window, X11Error> {
        unsafe {
            debug!("Opening X11 display");
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err(X11Error::OpenDisplayError);
            }

            debug!("Creating X11 window");
            let screen_id = XDefaultScreen(display);
            let mut window_attributes = x11::xlib::XSetWindowAttributes {
                background_pixmap: 0,
                background_pixel: 0,
                border_pixmap: 0,
                border_pixel: 0,
                bit_gravity: 0,
                win_gravity: 0,
                backing_store: 0,
                backing_planes: 0,
                backing_pixel: 0,
                save_under: 0,
                event_mask: x11::xlib::ExposureMask | x11::xlib::KeyPressMask,
                do_not_propagate_mask: 0,
                override_redirect: 0,
                colormap: 0,
                cursor: 0,
            };
            let window = x11::xlib::XCreateWindow(
                display,
                x11::xlib::XRootWindow(display, screen_id),
                0,
                0,
                self.config.width,
                self.config.height,
                0,
                x11::xlib::CopyFromParent as i32,
                x11::xlib::InputOutput as u32,
                x11::xlib::CopyFromParent as *mut _,
                x11::xlib::CWEventMask,
                addr_of_mut!(window_attributes),
            );
            if window == 0 {
                return Err(X11Error::CreateWindowError);
            }

            debug!("Setting up X11 window attributes");
            XStoreName(display, window, self.config.title.as_ptr() as *const c_char);
            XSync(display, 0);
            XAutoRepeatOff(display);
            XClearWindow(display, window);
            XMapRaised(display, window);

            debug!("Creating Vulkan graphics");
            let graphics = VulkanGraphics::new(VulkanGraphicsInitArgs {
                instance_extensions: vec![ash::khr::xlib_surface::NAME.as_ptr() as *const c_char],
                device_extensions: vec![],
                layers: vec![],
                surface_constructor: Box::new(|entry, instance| {
                    debug!("Creating X11 Vulkan surface");
                    let surface_loader = ash::khr::xlib_surface::Instance::new(entry, instance);
                    let create_info = vk::XlibSurfaceCreateInfoKHR {
                        dpy: display as *mut _,
                        window,
                        ..Default::default()
                    };
                    let surface = surface_loader
                        .create_xlib_surface(&create_info, None)
                        .map_err(VulkanGraphicsError::SurfaceCreateError)?;
                    Ok(surface)
                }),
            })
            .map_err(X11Error::GraphicsCreateError)?;

            info!("X11 Window with Vulkan graphics created successfully");
            Ok(X11Window {
                display,
                window,
                graphics,
            })
        }
    }
}

impl Window<X11Error, VulkanGraphics> for X11Window {
    fn tick(&mut self) -> Result<bool, X11Error> {
        let event = unsafe {
            let mut event: x11::xlib::XEvent = std::mem::zeroed();
            x11::xlib::XNextEvent(self.display, &mut event);
            event
        };

        match event.get_type() {
            x11::xlib::Expose => {
                debug!("Expose event received");
                // Handle expose event (e.g., redraw the window)
            }
            x11::xlib::KeyPress => {
                debug!("Key press event received");
                // Handle key press event
            }
            _ => {
                debug!("Unhandled event type: {}", event.get_type());
            }
        }

        /* Return true to indicate the window is still active */
        Ok(true)
    }

    fn get_graphics(&mut self) -> &mut VulkanGraphics {
        &mut self.graphics
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        unsafe {
            if !self.display.is_null() {
                debug!("Closing X11 display");
                x11::xlib::XCloseDisplay(self.display);
            }
        }
    }
}
