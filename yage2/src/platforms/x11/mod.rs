use crate::engine::application::{Application, ApplicationConfig, ApplicationError};
use crate::engine::event::{Event, KeyCode, MouseButton};
use crate::engine::graphics::Graphics;
use crate::engine::vulkan::{VulkanGraphics, VulkanGraphicsError, VulkanGraphicsInitArgs};
use crate::engine::window::{Window, WindowConfig, WindowFactory};
use ash::vk;
use log::{debug, info};
use std::ffi::c_char;
use std::os::raw::{c_uchar, c_uint};
use std::ptr::addr_of_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use x11::xlib;
use x11::xlib::{Atom, ButtonPress, ButtonPressMask, ButtonRelease, ButtonReleaseMask, CWEventMask, ClientMessage, CopyFromParent, CurrentTime, Display, Expose, ExposureMask, InputOutput, KeyPress, KeyPressMask, KeyRelease, KeyReleaseMask, MotionNotify, NoEventMask, PointerMotionMask, ShiftMask, XAutoRepeatOff, XClearWindow, XCloseDisplay, XCreateWindow, XDefaultScreen, XDestroyWindow, XEvent, XFlush, XInternAtom, XKeycodeToKeysym, XMapRaised, XNextEvent, XOpenDisplay, XRootWindow, XSendEvent, XSetWMProtocols, XSetWindowAttributes, XStoreName, XSync, XkbKeycodeToKeysym};

mod input;

#[derive(Debug)]
#[allow(dead_code)]
pub enum X11Error {
    OpenDisplayError,
    CreateWindowError,
    SpawnEventsThreadError,
    JoinEventsThreadError,
    GraphicsCreateError(VulkanGraphicsError),
    VulkanCreateSurfaceError(vk::Result),
    VulkanUpdateSurfaceError(VulkanGraphicsError),
}

pub struct X11Window {
    display: *mut Display,
    window: xlib::Window,
    graphics: VulkanGraphics,

    delete_message: Atom,

    /* A signal to stop the event handling thread */
    stop_signal: Arc<AtomicBool>,
    events_thread: Option<thread::JoinHandle<Result<(), X11Error>>>,
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

    fn get_window_factory(
        &self,
    ) -> Arc<dyn WindowFactory<X11Window, X11Error, VulkanGraphics> + Send + Sync> {
        Arc::new(self.window_factory.clone())
    }
}

#[derive(Clone, Debug)]
struct X11WindowFactory {
    config: WindowConfig,
}

fn process_events_sync(
    display: *mut Display,
    close_atom: Atom,
    events_sender: &Sender<Event>,
) -> Result<bool, X11Error> {
    let event = unsafe {
        let mut event: XEvent = std::mem::zeroed();
        XNextEvent(display, &mut event);
        event
    };

    match event.get_type() {
        ClientMessage => unsafe {
            debug!("Client message event received");
            let ptr = event.client_message.data.as_longs()[0];
            if ptr == close_atom as i64 {
                debug!("Window close requested via client message");
                return Ok(false);
            } else {
                debug!("Unhandled client message: {}", ptr);
            }
        },

        Expose => {
            // Handle expose event (e.g., redraw the window)
        }

        KeyPress => {
            let keycode = unsafe { event.key.keycode };
            let keystate = unsafe { event.key.state };
            let key = input::convert_key(display, keycode, keystate);
            if let Err(e) = events_sender.send(Event::KeyPress(key)) {
                debug!("Failed to send KeyPress event: {:?}", e);
            }
        }

        KeyRelease => {
            let keycode = unsafe { event.key.keycode };
            let keystate = unsafe { event.key.state };
            let key = input::convert_key(display, keycode, keystate);
            if let Err(e) = events_sender.send(Event::KeyRelease(key)) {
                debug!("Failed to send KeyRelease event: {:?}", e);
            }
        }

        ButtonPress => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            if let Err(e) = events_sender.send(Event::MouseButtonPress(mouse_button)) {
                debug!("Failed to send MouseButtonPress event: {:?}", e);
            }
        }

        ButtonRelease => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            if let Err(e) = events_sender.send(Event::MouseButtonRelease(mouse_button)) {
                debug!("Failed to send MouseButtonRelease event: {:?}", e);
            }
        }

        MotionNotify => {
            let x = unsafe { event.motion.x };
            let y = unsafe { event.motion.y };
            if let Err(e) = events_sender.send(Event::MouseMove {
                x: x as f32,
                y: y as f32,
            }) {
                debug!("Failed to send MouseMove event: {:?}", e);
            }
        }

        _ => {
            debug!("Unhandled event type: {}", event.get_type());
        }
    }

    Ok(true)
}

impl WindowFactory<X11Window, X11Error, VulkanGraphics> for X11WindowFactory {
    fn new(config: WindowConfig) -> Result<Self, X11Error>
    where
        Self: Sized,
    {
        info!("Creating X11 window factory with config: {:?}", config);
        Ok(X11WindowFactory { config })
    }

    fn create_window(&self, events_sender: Sender<Event>) -> Result<X11Window, X11Error> {
        unsafe {
            debug!("Opening X11 display");
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err(X11Error::OpenDisplayError);
            }

            debug!("Creating X11 window");
            let screen_id = XDefaultScreen(display);
            let mut window_attributes = XSetWindowAttributes {
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
                event_mask: ExposureMask
                    | KeyPressMask
                    | KeyReleaseMask
                    | ButtonPressMask
                    | ButtonReleaseMask
                    | PointerMotionMask,
                do_not_propagate_mask: 0,
                override_redirect: 0,
                colormap: 0,
                cursor: 0,
            };
            let window = XCreateWindow(
                display,
                XRootWindow(display, screen_id),
                0,
                0,
                self.config.width,
                self.config.height,
                0,
                CopyFromParent as i32,
                InputOutput as u32,
                CopyFromParent as *mut _,
                CWEventMask,
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

            let delete_message =
                XInternAtom(display, b"WM_DELETE_WINDOW\0".as_ptr() as *const c_char, 0);
            XSetWMProtocols(display, window, &delete_message as *const _ as *mut _, 1);

            let stop_signal = Arc::new(AtomicBool::new(false));

            let signal_stop = stop_signal.clone();
            let display_ptr = display.addr();
            let events_thread = thread::Builder::new()
                .name("X11Events".to_string())
                .spawn(move || {
                    debug!("Starting X11 events thread");
                    let display = &mut *(display_ptr as *mut Display);
                    let queue = events_sender.clone();
                    while !signal_stop.load(Ordering::Relaxed) {
                        match process_events_sync(display, delete_message, &queue) {
                            Ok(should_continue) => {
                                if !should_continue {
                                    debug!("Stopping X11 events thread");
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Error processing X11 events: {:?}", e);
                                return Err(e);
                            }
                        }
                    }

                    debug!("X11 events thread exiting");
                    Ok(())
                })
                .map_err(|e| {
                    debug!("Failed to spawn X11 events thread: {:?}", e);
                    X11Error::SpawnEventsThreadError
                })?;

            info!("X11 Window with Vulkan graphics created successfully");
            Ok(X11Window {
                display,
                window,
                graphics,
                delete_message,
                stop_signal: stop_signal.clone(),
                events_thread: Some(events_thread),
            })
        }
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        debug!("Dropping X11Window");
        unsafe {
            debug!("Destroying X11 window");
            XDestroyWindow(self.display, self.window);

            if !self.display.is_null() {
                debug!("Closing X11 display");
                XCloseDisplay(self.display);
            }
        }
    }
}

impl Window<X11Error, VulkanGraphics> for X11Window {
    fn tick(&mut self) -> Result<bool, X11Error> {
        /* if the events thread is dead, we need to stop as well */
        if let Some(ref thread) = self.events_thread {
            if thread.is_finished() {
                debug!("X11 events thread is dead, stopping window tick");
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn kill(&mut self) -> Result<(), X11Error> {
        /* If the events thread is running,
         * signal it to stop */
        if let Some(thread) = self.events_thread.take() {
            self.stop_signal.store(true, Ordering::Relaxed);

            /* Send event to stop the event handling thread */
            let event: XEvent = unsafe {
                let mut event: XEvent = std::mem::zeroed();
                event.type_ = ClientMessage;
                event.client_message.window = self.window;
                event.client_message.message_type =
                    XInternAtom(self.display, b"WM_PROTOCOLS\0".as_ptr() as *const c_char, 1);
                event.client_message.format = 32;
                event
                    .client_message
                    .data
                    .set_long(0, self.delete_message as i64); // Use the delete message atom
                event.client_message.data.set_long(1, CurrentTime as i64); // Use CurrentTime to indicate the time of the event
                event
            };

            unsafe {
                XSendEvent(
                    self.display,
                    self.window,
                    0, // False for no propagation
                    NoEventMask,
                    &event as *const XEvent as *mut XEvent,
                );
                XSync(self.display, 0);
                XFlush(self.display);
            }

            /* Wait for the events thread to finish */
            debug!("Waiting for X11 events thread to finish");
            thread
                .join()
                .map_err(|_| X11Error::JoinEventsThreadError)??;
        }

        Ok(())
    }

    fn get_graphics(&mut self) -> &mut VulkanGraphics {
        &mut self.graphics
    }
}
