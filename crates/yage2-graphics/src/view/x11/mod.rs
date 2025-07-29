use crate::event::Event;
use crate::view::{TickResult, ViewConfig, ViewTrait};
use crate::vulkan::objects::surface::{Surface, ViewHandle};
use crate::vulkan::GraphicsError;
use ash::{vk, Entry, Instance};
use log::{debug, info};
use std::ffi::{c_char, c_uint};
use std::ptr::addr_of_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use x11::xlib;
use x11::xlib::{
    Atom, ButtonPressMask, ButtonReleaseMask, CWEventMask, ClientMessage, CopyFromParent,
    CurrentTime, Display, ExposureMask, InputOutput, KeyPressMask, KeyReleaseMask, NoEventMask,
    PointerMotionMask, XAutoRepeatOff, XClearWindow, XCloseDisplay, XCreateWindow, XDefaultScreen,
    XDestroyWindow, XEvent, XFlush, XInternAtom, XMapRaised, XNextEvent, XOpenDisplay, XRootWindow,
    XSendEvent, XSetWMProtocols, XSetWindowAttributes, XStoreName, XSync,
};

mod input;

#[derive(Clone, Debug)]
pub struct PlatformSpecificViewConfig {}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ViewError {
    OpenDisplayError,
    CreateWindowError,
    SpawnEventsThreadError,
    JoinEventsThreadError,
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewError::OpenDisplayError => write!(f, "Failed to open X11 display"),
            ViewError::CreateWindowError => write!(f, "Failed to create X11 window"),
            ViewError::SpawnEventsThreadError => write!(f, "Failed to spawn events thread"),
            ViewError::JoinEventsThreadError => write!(f, "Failed to join events thread"),
        }
    }
}

impl std::error::Error for ViewError {}

pub(crate) struct View {
    display: *mut Display,
    window: xlib::Window,

    delete_message: Atom,

    /* A signal to stop the event handling thread */
    stop_signal: Arc<AtomicBool>,
    events_thread: Option<thread::JoinHandle<Result<(), ViewError>>>,
}

fn process_events_sync(
    display: *mut Display,
    close_atom: Atom,
    events_sender: &Sender<Event>,
) -> Result<bool, ViewError> {
    let event = unsafe {
        let mut event: XEvent = std::mem::zeroed();
        XNextEvent(display, &mut event);
        event
    };

    match event.get_type() {
        xlib::ClientMessage => unsafe {
            debug!("Client message event received");
            let ptr = event.client_message.data.as_longs()[0];
            if ptr == close_atom as i64 {
                debug!("Window close requested via client message");
                return Ok(false);
            } else {
                debug!("Unhandled client message: {}", ptr);
            }
        },

        xlib::Expose => {
            // Handle expose event (e.g., redraw the window)
        }

        xlib::KeyPress => {
            let keycode = unsafe { event.key.keycode };
            let keystate = unsafe { event.key.state };
            let key = input::convert_key(display, keycode, keystate);
            if let Err(e) = events_sender.send(Event::KeyPress(key)) {
                debug!("Failed to send KeyPress event: {:?}", e);
            }
        }

        xlib::KeyRelease => {
            let keycode = unsafe { event.key.keycode };
            let keystate = unsafe { event.key.state };
            let key = input::convert_key(display, keycode, keystate);
            if let Err(e) = events_sender.send(Event::KeyRelease(key)) {
                debug!("Failed to send KeyRelease event: {:?}", e);
            }
        }

        xlib::ButtonPress => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            if let Err(e) = events_sender.send(Event::MouseButtonPress(mouse_button)) {
                debug!("Failed to send MouseButtonPress event: {:?}", e);
            }
        }

        xlib::ButtonRelease => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            if let Err(e) = events_sender.send(Event::MouseButtonRelease(mouse_button)) {
                debug!("Failed to send MouseButtonRelease event: {:?}", e);
            }
        }

        xlib::MotionNotify => {
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

impl ViewTrait for View {
    fn open(cfg: ViewConfig, events_sender: Sender<Event>) -> Result<Self, ViewError> {
        unsafe {
            debug!("Opening X11 display");
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err(ViewError::OpenDisplayError);
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
                cfg.width as c_uint,
                cfg.height as c_uint,
                0,
                CopyFromParent as i32,
                InputOutput as u32,
                CopyFromParent as *mut _,
                CWEventMask,
                addr_of_mut!(window_attributes),
            );
            if window == 0 {
                return Err(ViewError::CreateWindowError);
            }

            debug!("Setting up X11 window attributes");
            XStoreName(display, window, cfg.title.as_ptr() as *const c_char);
            XSync(display, 0);
            XAutoRepeatOff(display);
            XClearWindow(display, window);
            XMapRaised(display, window);

            let delete_message =
                XInternAtom(display, b"WM_DELETE_WINDOW\0".as_ptr() as *const c_char, 0);
            XSetWMProtocols(display, window, &delete_message as *const _ as *mut _, 1);

            let stop_signal = Arc::new(AtomicBool::new(false));

            let signal_stop = stop_signal.clone();
            let display_ptr = display.addr();

            // TODO: Use thread manager
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
                    ViewError::SpawnEventsThreadError
                })?;

            info!("X11 Window with Vulkan graphics created successfully");
            Ok(View {
                display,
                window,
                delete_message,
                stop_signal: stop_signal.clone(),
                events_thread: Some(events_thread),
            })
        }
    }

    fn get_handle(&self) -> ViewHandle {
        ViewHandle::X11 {
            display: self.display as *mut vk::Display,
            window: self.window as vk::Window,
        }
    }

    fn tick(&mut self) -> TickResult {
        /* if the events thread is dead, we need to stop as well */
        if let Some(ref thread) = self.events_thread {
            if thread.is_finished() {
                debug!("X11 events thread is dead, stopping window tick");
                return TickResult::Closed;
            }
        }

        TickResult::Continue
    }

    fn set_size(&self, width: usize, height: usize) {
        todo!()
    }

    fn set_title(&self, title: &str) {
        todo!()
    }
}

impl Drop for View {
    fn drop(&mut self) {
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
            let _ = thread.join().map_err(|_| ViewError::JoinEventsThreadError);
        }

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
