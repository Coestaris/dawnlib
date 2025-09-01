use crate::gl::ViewHandleOpenGL;
use crate::input::InputEvent;
use crate::view::{TickResult, ViewConfig, ViewCursor, ViewGeometry, ViewTrait};
use crossbeam_channel::Sender;
use log::{debug, info, warn};
use std::ffi::{c_char, c_int, c_uint};
use std::ptr::addr_of_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use x11::glx::{
    glXChooseFBConfig, glXCreateNewContext, glXGetFBConfigAttrib, glXGetProcAddress,
    glXGetVisualFromFBConfig, glXIsDirect, glXMakeCurrent, glXQueryVersion, glXSwapBuffers,
    GLXContext, GLXFBConfig,
};
use x11::xlib;
use x11::xlib::{
    Atom, ButtonPressMask, ButtonReleaseMask, CWColormap, CWEventMask, ClientMessage,
    ConfigureNotify, CopyFromParent, CurrentTime, Display, ExposureMask, InputOutput, KeyPressMask,
    KeyReleaseMask, NoEventMask, PointerMotionMask, StructureNotifyMask, Visual, XAutoRepeatOff,
    XAutoRepeatOn, XChangeProperty, XClearWindow, XCloseDisplay, XCreateColormap, XCreateWindow,
    XDefaultScreen, XDefineCursor, XDestroyWindow, XEvent, XFlush, XFree, XFreeColormap,
    XInternAtom, XMapRaised, XMapWindow, XNextEvent, XOpenDisplay, XRootWindow, XSendEvent,
    XSetWMProtocols, XSetWindowAttributes, XStoreName, XSync, XVisualInfo,
};

mod input;

#[derive(Clone, Debug)]
pub struct PlatformSpecificViewConfig {}

#[derive(Clone, Debug)]
pub enum ViewError {
    OpenDisplayError,
    CreateWindowError,
    SpawnEventsThreadError,
    JoinEventsThreadError,
    #[cfg(feature = "gl")]
    GLXError(String),
}

impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewError::OpenDisplayError => write!(f, "Failed to open X11 display"),
            ViewError::CreateWindowError => write!(f, "Failed to create X11 window"),
            ViewError::SpawnEventsThreadError => write!(f, "Failed to spawn events thread"),
            ViewError::JoinEventsThreadError => write!(f, "Failed to join events thread"),
            #[cfg(feature = "gl")]
            ViewError::GLXError(msg) => write!(f, "GLX error: {}", msg),
        }
    }
}

impl std::error::Error for ViewError {}

pub(crate) struct View {
    display: *mut Display,
    window: xlib::Window,
    fb_config: GLXFBConfig,
    color_map: xlib::Colormap,

    delete_message: Atom,

    /* A signal to stop the event handling thread */
    stop_signal: Arc<AtomicBool>,
    events_thread: Option<thread::JoinHandle<Result<(), ViewError>>>,
}

fn process_events_sync(
    display: *mut Display,
    close_atom: Atom,
    events_sender: &Sender<InputEvent>,
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
            events_sender.send(InputEvent::KeyPress(key)).unwrap();
        }

        xlib::KeyRelease => {
            let keycode = unsafe { event.key.keycode };
            let keystate = unsafe { event.key.state };
            let key = input::convert_key(display, keycode, keystate);
            events_sender.send(InputEvent::KeyRelease(key)).unwrap();
        }

        xlib::ButtonPress => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            events_sender
                .send(InputEvent::MouseButtonPress(mouse_button))
                .unwrap();
        }

        xlib::ButtonRelease => {
            let button = unsafe { event.button.button };
            let mouse_button = input::convert_mouse(button);
            events_sender
                .send(InputEvent::MouseButtonRelease(mouse_button))
                .unwrap();
        }

        // If the window is resized, we need to update the size of the window
        xlib::ConfigureNotify => {
            let width = unsafe { event.configure.width };
            let height = unsafe { event.configure.height };
            events_sender
                .send(InputEvent::Resize {
                    width: width as usize,
                    height: height as usize,
                })
                .unwrap();
        }

        xlib::MotionNotify => {
            let x = unsafe { event.motion.x };
            let y = unsafe { event.motion.y };
            events_sender
                .send(InputEvent::MouseMove {
                    x: x as f32,
                    y: y as f32,
                })
                .unwrap();
        }

        _ => {
            debug!("Unhandled event type: {}", event.get_type());
        }
    }

    Ok(true)
}

#[cfg(feature = "gl")]
fn select_fb(display: *mut Display) -> Result<((GLXFBConfig, *mut XVisualInfo)), ViewError> {
    unsafe {
        #[rustfmt::skip]
        static FB_ATTRIBS: [i32; 27] = [
            x11::glx::GLX_X_RENDERABLE, 1,
            x11::glx::GLX_DRAWABLE_TYPE, x11::glx::GLX_WINDOW_BIT,
            x11::glx::GLX_RENDER_TYPE, x11::glx::GLX_RGBA_BIT,
            x11::glx::GLX_X_VISUAL_TYPE, x11::glx::GLX_TRUE_COLOR,
            x11::glx::GLX_RED_SIZE, 8,
            x11::glx::GLX_GREEN_SIZE, 8,
            x11::glx::GLX_BLUE_SIZE, 8,
            x11::glx::GLX_ALPHA_SIZE, 8,
            x11::glx::GLX_DEPTH_SIZE, 24,
            x11::glx::GLX_STENCIL_SIZE, 8,
            x11::glx::GLX_DOUBLEBUFFER, 1,
            x11::glx::GLX_SAMPLE_BUFFERS, 1, // <-- MSAA
            x11::glx::GLX_SAMPLES, 4, // <-- MSAA
            0, // Terminate the list of attributes
        ];

        let (mut gl_major, mut gl_minor) = (0, 0);
        if glXQueryVersion(display, addr_of_mut!(gl_major), addr_of_mut!(gl_minor)) == 0 {
            return Err(ViewError::GLXError(
                "Failed to query GLX version".to_string(),
            ));
        }
        info!("GLX version: {}.{}", gl_major, gl_minor);

        if (gl_major, gl_minor) < (1, 3) {
            return Err(ViewError::GLXError("GLX version too low".to_string()));
        }

        let mut fb_count = 0;
        let fb_configs = glXChooseFBConfig(
            display,
            XDefaultScreen(display),
            FB_ATTRIBS.as_ptr(),
            addr_of_mut!(fb_count),
        );
        if fb_configs.is_null() || fb_count <= 0 {
            return Err(ViewError::GLXError(
                "Failed to choose FB config".to_string(),
            ));
        }

        info!("Selected {} framebuffer configurations", fb_count);
        let (mut best_fbc_index, mut worst_fbc_index, mut best_num_samp, mut worst_num_samp) =
            (-1, -1, -1, 999);

        for i in 0..fb_count {
            let visual = glXGetVisualFromFBConfig(display, *fb_configs.add(i as usize));
            if visual.is_null() {
                warn!("Failed to get visual from FBConfig at index {}", i);
                continue;
            }

            let (mut samp_buf, mut samples) = (0, 0);
            glXGetFBConfigAttrib(
                display,
                *fb_configs.add(i as usize),
                x11::glx::GLX_SAMPLE_BUFFERS,
                addr_of_mut!(samp_buf),
            );
            glXGetFBConfigAttrib(
                display,
                *fb_configs.add(i as usize),
                x11::glx::GLX_SAMPLES,
                addr_of_mut!(samples),
            );

            info!(
                "FBConfig[{}]: Visual ID: {}, Sample Buffers: {}, Samples: {}",
                i,
                (*visual).visualid,
                samp_buf,
                samples
            );

            if best_fbc_index < 0 || (samp_buf != 0 && samples > best_num_samp) {
                best_fbc_index = i;
                best_num_samp = samples;
            }
            if worst_fbc_index < 0 || samp_buf == 0 || samples < worst_num_samp {
                worst_fbc_index = i;
                worst_num_samp = samples;
            }

            XFree(visual as *mut _);
        }

        let best_fbc = *fb_configs.add(best_fbc_index as usize);
        XFree(fb_configs as *mut _);

        info!(
            "Best FBConfig: Index: {}, Sample Buffers: {}, Samples: {}",
            best_fbc_index, best_num_samp, worst_num_samp
        );

        // Get visual info for the best FBConfig
        let visual_info_ptr = glXGetVisualFromFBConfig(display, best_fbc);
        if visual_info_ptr.is_null() {
            return Err(ViewError::GLXError(
                "Failed to get visual info from FBConfig".to_string(),
            ));
        }

        Ok((best_fbc, visual_info_ptr))
    }
}

impl ViewTrait for View {
    fn open(cfg: ViewConfig, events_sender: Sender<InputEvent>) -> Result<Self, ViewError> {
        unsafe {
            debug!("Opening X11 display");
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err(ViewError::OpenDisplayError);
            }

            let mut screen_id = XDefaultScreen(display);
            let mut color_map = 0;
            let mut visual_info: *mut XVisualInfo = std::ptr::null_mut();
            let mut fb_config: GLXFBConfig = std::ptr::null_mut();

            #[cfg(feature = "gl")]
            {
                let (fbc, vi) = select_fb(display)?;
                screen_id = (*vi).screen;
                visual_info = vi;
                fb_config = fbc;

                color_map = XCreateColormap(
                    display,
                    XRootWindow(display, screen_id),
                    vi as *mut _,
                    xlib::AllocNone,
                );
                if color_map == 0 {
                    return Err(ViewError::GLXError("Failed to create colormap".to_string()));
                }
            }

            debug!("Creating X11 window");
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
                    | StructureNotifyMask
                    | PointerMotionMask,
                do_not_propagate_mask: 0,
                override_redirect: 0,
                colormap: color_map,
                cursor: 0,
            };
            let window = XCreateWindow(
                display,
                XRootWindow(display, screen_id),
                0,
                0,
                800,
                600,
                0,
                if visual_info.is_null() {
                    CopyFromParent as c_int
                } else {
                    (*visual_info).depth
                },
                InputOutput as u32,
                if visual_info.is_null() {
                    CopyFromParent as *mut Visual
                } else {
                    (*visual_info).visual
                },
                CWEventMask | if color_map != 0 { CWColormap } else { 0 },
                addr_of_mut!(window_attributes),
            );
            if window == 0 {
                return Err(ViewError::CreateWindowError);
            }

            XMapWindow(display, window);

            // Destroy the visual info if it was created
            if !visual_info.is_null() {
                XFree(visual_info as *mut _);
            }

            debug!("Setting up X11 window attributes");
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

            let events_thread = thread::Builder::new()
                .name("x11events".to_string())
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

            info!("X11 Window created successfully");
            let mut view = View {
                display,
                window,
                fb_config,
                color_map,
                delete_message,
                stop_signal: stop_signal.clone(),
                events_thread: Some(events_thread),
            };

            view.set_geometry(cfg.geometry)?;
            view.set_title(&cfg.title)?;
            view.set_cursor(cfg.cursor)?;
            Ok(view)
        }
    }

    fn get_handle(&self) -> ViewHandle {
        ViewHandle {
            display: self.display,
            window: self.window,
            fbc: self.fb_config,
            ctx: None,
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

    fn set_geometry(&mut self, geometry: ViewGeometry) -> Result<(), crate::view::ViewError> {
        match geometry {
            ViewGeometry::Normal(width, height) => {
                unsafe {
                    xlib::XResizeWindow(
                        self.display,
                        self.window,
                        width as c_uint,
                        height as c_uint,
                    );
                    XSync(self.display, 0);
                }
                Ok(())
            }
            ViewGeometry::BorderlessFullscreen => {
                unimplemented!()
            }
            ViewGeometry::Fullscreen => {
                unimplemented!()
            }
        }
    }
    fn set_title(&mut self, title: &str) -> Result<(), crate::view::ViewError> {
        unsafe {
            XStoreName(self.display, self.window, title.as_ptr() as *const c_char);
            XSync(self.display, 0);
        }

        Ok(())
    }
    fn set_cursor(&mut self, _cursor: ViewCursor) -> Result<(), crate::view::ViewError> {
        unsafe {
            XDefineCursor(self.display, self.window, 0 as xlib::Cursor);
            XSync(self.display, 0);
        }

        Ok(())
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

        unsafe {
            debug!("Destroying X11 window");
            XAutoRepeatOn(self.display);

            XDestroyWindow(self.display, self.window);

            if self.color_map != 0 {
                debug!("Freeing X11 colormap");
                XFreeColormap(self.display, self.color_map);
            }

            if !self.display.is_null() {
                debug!("Closing X11 display");
                XCloseDisplay(self.display);
            }
        }
    }
}

pub struct ViewHandle {
    display: *mut Display,
    window: xlib::Window,
    fbc: GLXFBConfig,
    #[cfg(feature = "gl")]
    ctx: Option<GLXContext>,
}

impl ViewHandle {
    pub fn error_box(title: &str, message: &str) {
        todo!()
    }
}

#[cfg(feature = "gl")]
impl ViewHandleOpenGL for ViewHandle {
    fn create_context(&mut self, fps: usize, vsync: bool) -> Result<(), crate::view::ViewError> {
        unsafe {
            debug!("Creating GLX context");
            let ctx = glXCreateNewContext(
                self.display,
                self.fbc,
                x11::glx::GLX_RGBA_TYPE as c_int,
                std::ptr::null_mut(),
                1, // Direct rendering
            );

            if ctx.is_null() {
                return Err(ViewError::GLXError(
                    "Failed to create GLX context".to_string(),
                ));
            }

            XSync(self.display, 0);

            self.ctx = Some(ctx);

            // Make sure that context is in direct rendering mode
            if glXIsDirect(self.display, ctx) == 0 {
                warn!("GLX context is not in direct rendering mode");
            } else {
                info!("GLX context is in direct rendering mode");
            }

            // Make the context current
            if glXMakeCurrent(self.display, self.window, ctx) == 0 {
                return Err(ViewError::GLXError(
                    "Failed to make GLX context current".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn get_proc_addr(
        &mut self,
        symbol: &str,
    ) -> Result<*const std::ffi::c_void, crate::view::ViewError> {
        unsafe {
            let c_symbol = std::ffi::CString::new(symbol)
                .map_err(|_| ViewError::GLXError("Invalid symbol name".to_string()))?;

            let addr = glXGetProcAddress(c_symbol.as_ptr() as *const u8);
            if addr.is_none() {
                return Err(ViewError::GLXError(format!(
                    "Failed to get address for symbol: {}",
                    symbol
                )));
            }

            Ok(addr.unwrap() as *const std::ffi::c_void)
        }
    }

    fn swap_buffers(&self) -> Result<(), crate::view::ViewError> {
        unsafe {
            glXSwapBuffers(self.display, self.window);

            Ok(())
        }
    }
}

#[cfg(feature = "gl")]
impl Drop for ViewHandle {
    fn drop(&mut self) {
        unsafe {
            info!("Destroying GLX context");
            if let Some(ctx) = self.ctx.take() {
                x11::glx::glXDestroyContext(self.display, ctx);
            }
        }
    }
}
