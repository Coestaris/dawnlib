use crate::gl::bindings;
use crate::gl::bindings::types::{GLboolean, GLchar, GLenum, GLsizei, GLuint};
use log::info;
use std::ffi::{c_void};

#[derive(Debug, Clone)]
pub(crate) enum MessageSource {
    Api,
    WindowSystem,
    ShaderCompiler,
    ThirdParty,
    Application,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) enum MessageType {
    Error,
    DeprecatedBehavior,
    UndefinedBehavior,
    Portability,
    Performance,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) enum MessageSeverity {
    High,
    Medium,
    Low,
    Notification,
    Other,
}

impl MessageSource {
    pub fn new(source: GLenum) -> Self {
        match source {
            bindings::DEBUG_SOURCE_API => MessageSource::Api,
            bindings::DEBUG_SOURCE_WINDOW_SYSTEM => MessageSource::WindowSystem,
            bindings::DEBUG_SOURCE_SHADER_COMPILER => MessageSource::ShaderCompiler,
            bindings::DEBUG_SOURCE_THIRD_PARTY => MessageSource::ThirdParty,
            bindings::DEBUG_SOURCE_APPLICATION => MessageSource::Application,
            _ => MessageSource::Other,
        }
    }
}

impl std::fmt::Display for MessageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_str = match self {
            MessageSource::Api => "API",
            MessageSource::WindowSystem => "Window System",
            MessageSource::ShaderCompiler => "Shader Compiler",
            MessageSource::ThirdParty => "Third Party",
            MessageSource::Application => "Application",
            MessageSource::Other => "Other",
        };
        write!(f, "{}", source_str)
    }
}

impl MessageType {
    pub fn new(gltype: GLenum) -> Self {
        match gltype {
            bindings::DEBUG_TYPE_ERROR => MessageType::Error,
            bindings::DEBUG_TYPE_DEPRECATED_BEHAVIOR => MessageType::DeprecatedBehavior,
            bindings::DEBUG_TYPE_UNDEFINED_BEHAVIOR => MessageType::UndefinedBehavior,
            bindings::DEBUG_TYPE_PORTABILITY => MessageType::Portability,
            bindings::DEBUG_TYPE_PERFORMANCE => MessageType::Performance,
            _ => MessageType::Other,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_str = match self {
            MessageType::Error => "Error",
            MessageType::DeprecatedBehavior => "Deprecated Behavior",
            MessageType::UndefinedBehavior => "Undefined Behavior",
            MessageType::Portability => "Portability",
            MessageType::Performance => "Performance",
            MessageType::Other => "Other",
        };
        write!(f, "{}", type_str)
    }
}

impl MessageSeverity {
    pub fn new(severity: GLenum) -> Self {
        match severity {
            bindings::DEBUG_SEVERITY_HIGH => MessageSeverity::High,
            bindings::DEBUG_SEVERITY_MEDIUM => MessageSeverity::Medium,
            bindings::DEBUG_SEVERITY_LOW => MessageSeverity::Low,
            bindings::DEBUG_SEVERITY_NOTIFICATION => MessageSeverity::Notification,
            _ => MessageSeverity::Other,
        }
    }
}

impl std::fmt::Display for MessageSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity_str = match self {
            MessageSeverity::High => "High",
            MessageSeverity::Medium => "Medium",
            MessageSeverity::Low => "Low",
            MessageSeverity::Notification => "Notification",
            MessageSeverity::Other => "Other",
        };
        write!(f, "{}", severity_str)
    }
}

type Callback = dyn Fn(MessageSource, MessageType, MessageSeverity, &str) + 'static;

pub(crate) struct Debugger {
    holder: *mut Box<Callback>,
}

impl Drop for Debugger {
    fn drop(&mut self) {
        info!("Disabling OpenGL debug output");

        if !self.holder.is_null() {
            unsafe {
                // Disable OpenGL debug output
                bindings::DebugMessageCallback(None, std::ptr::null_mut());
                bindings::Disable(bindings::DEBUG_OUTPUT);

                // Restore the outer Box from the raw pointer
                // Inner and the outer now will be dropped automatically
                let _outer: Box<Box<Callback>> = Box::from_raw(self.holder);
                self.holder = std::ptr::null_mut(); // Prevent double free
            }
        }
    }
}

impl Debugger {
    pub(crate) unsafe fn new<F>(callback: F) -> Self
    where
        F: Fn(MessageSource, MessageType, MessageSeverity, &str) + Send + Sync + 'static,
    {
        info!("Enabling OpenGL debug output");

        // Internal box with trait object (fat pointer)
        let inner: Box<Callback> = Box::new(callback);
        // Outer box to hold the inner Box (thin pointer)
        // Convert the outer Box to a raw pointer. At this point
        // we must manually manage the memory.
        // This is safe because we will ensure to drop it in the destructor.
        let holder = Box::into_raw(Box::new(inner));

        extern "system" fn dbg_proc(
            source: GLenum,
            gltype: GLenum,
            _id: GLuint,
            severity: GLenum,
            length: GLsizei,
            message: *const GLchar,
            user_param: *mut c_void,
        ) {
            let source = MessageSource::new(source);
            let message_type = MessageType::new(gltype);
            let severity = MessageSeverity::new(severity);

            let bytes = unsafe { std::slice::from_raw_parts(message as *const u8, length as usize) };
            let msg = std::borrow::Cow::from(String::from_utf8_lossy(bytes)); // Cow<'_, str>

            // Restore the outer Box from the raw pointer
            let outer: &Box<Callback> = unsafe { &*(user_param as *mut Box<Callback>) };
            // Convert the outer Box to a reference to the inner Box
            let f = &**outer;
            // Call stored callback function
            f(source, message_type, severity, &*msg);
        }

        // Enable OpenGL debug output
        bindings::Enable(bindings::DEBUG_OUTPUT);
        bindings::Enable(bindings::DEBUG_OUTPUT_SYNCHRONOUS);
        // Pass boxed function as user ctx
        bindings::DebugMessageCallback(Some(dbg_proc), holder.cast());
        bindings::DebugMessageControl(
            bindings::DONT_CARE, // All sources
            bindings::DONT_CARE, // All types
            bindings::DONT_CARE, // All severities
            0,
            std::ptr::null(),
            GLboolean::from(true),
        );

        Debugger { holder }
    }
}
