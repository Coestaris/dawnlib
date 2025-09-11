use glow::HasContext;
use log::warn;

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
    pub fn new(source: u32) -> Self {
        match source {
            glow::DEBUG_SOURCE_API => MessageSource::Api,
            glow::DEBUG_SOURCE_WINDOW_SYSTEM => MessageSource::WindowSystem,
            glow::DEBUG_SOURCE_SHADER_COMPILER => MessageSource::ShaderCompiler,
            glow::DEBUG_SOURCE_THIRD_PARTY => MessageSource::ThirdParty,
            glow::DEBUG_SOURCE_APPLICATION => MessageSource::Application,
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
    pub fn new(gltype: u32) -> Self {
        match gltype {
            glow::DEBUG_TYPE_ERROR => MessageType::Error,
            glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => MessageType::DeprecatedBehavior,
            glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => MessageType::UndefinedBehavior,
            glow::DEBUG_TYPE_PORTABILITY => MessageType::Portability,
            glow::DEBUG_TYPE_PERFORMANCE => MessageType::Performance,
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
    pub fn new(severity: u32) -> Self {
        match severity {
            glow::DEBUG_SEVERITY_HIGH => MessageSeverity::High,
            glow::DEBUG_SEVERITY_MEDIUM => MessageSeverity::Medium,
            glow::DEBUG_SEVERITY_LOW => MessageSeverity::Low,
            glow::DEBUG_SEVERITY_NOTIFICATION => MessageSeverity::Notification,
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

pub unsafe fn setup_debug_callback<F>(gl: &mut glow::Context, f: F)
where
    F: Fn(MessageSource, MessageType, MessageSeverity, &str) + 'static + Send + Sync,
{
    #[cfg(target_os = "macos")]
    {
        // Maximum supported OpenGL version on macOS is 4.1
        // Debug output is not available in this version
        // So we just log a warning and return
        warn!("Debug output is not supported on macOS with OpenGL 4.1");
        gl.enable(glow::DEBUG_OUTPUT);
        gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
        return;
    }

    #[cfg(not(target_os = "macos"))]
    {
        gl.debug_message_callback(
            move |source: u32, msg_type: u32, id: u32, severity: u32, msg| {
                let source = MessageSource::new(source);
                let message_type = MessageType::new(msg_type);
                let severity = MessageSeverity::new(severity);

                f(source, message_type, severity, msg);
            },
        );
        // Filter out notifications
        gl.debug_message_control(
            glow::DONT_CARE,
            glow::DONT_CARE,
            glow::DEBUG_SEVERITY_NOTIFICATION,
            &[],
            false,
        );
        gl.enable(glow::DEBUG_OUTPUT);
        gl.enable(glow::DEBUG_OUTPUT_SYNCHRONOUS);
    }
}
