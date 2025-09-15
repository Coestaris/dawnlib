use glam::UVec2;
use std::sync::Arc;
use thiserror::Error;
use web_sys::wasm_bindgen::JsCast;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

pub struct Context {
    gl: Arc<glow::Context>,
    webgl: web_sys::WebGl2RenderingContext,
    canvas: web_sys::HtmlCanvasElement,
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Winit error: {0}")]
    Winit(#[from] winit::error::OsError),

    #[error("Canvas element is missing")]
    CanvasMissing,

    #[error("WebGL2 is not supported by this browser")]
    WebGl2NotSupported,

    #[error("JS error: {0}")]
    Js(String),
}

impl Context {
    pub fn create_contextual_window(
        attributes: WindowAttributes,
        event_loop: &ActiveEventLoop,
    ) -> Result<(Window, Context), ContextError> {
        use winit::platform::web::WindowExtWebSys;

        let window = event_loop.create_window(attributes)?;
        let canvas = window.canvas().ok_or(ContextError::CanvasMissing)?;

        {
            let web_window =
                web_sys::window().ok_or_else(|| ContextError::Js("no window".into()))?;
            let document = web_window
                .document()
                .ok_or_else(|| ContextError::Js("no document".into()))?;
            canvas
                .style()
                .set_property("display", "block")
                .map_err(|e| ContextError::Js(format!("{e:?}")))?;

            if let Some(body) = document.body() {
                if !body.contains(Some(&canvas)) {
                    body.append_child(&canvas)
                        .map_err(|e| ContextError::Js(format!("{e:?}")))?;
                }
            }
        }

        let webgl = {
            let ctx = canvas
                .get_context("webgl2")
                .map_err(|e| ContextError::Js(format!("{e:?}")))?
                .ok_or(ContextError::WebGl2NotSupported)?;
            ctx.dyn_into::<web_sys::WebGl2RenderingContext>()
                .map_err(|_| ContextError::WebGl2NotSupported)?
        };

        let gl = unsafe { glow::Context::from_webgl2_context(webgl.clone()) };

        let ctx = Context {
            gl: Arc::new(gl),
            webgl,
            canvas,
        };
        Ok((window, ctx))
    }

    pub fn resize(&self, size: UVec2) {
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let width = (size.x as f64 * dpr).round() as u32;
        let height = (size.y as f64 * dpr).round() as u32;

        self.canvas.set_width(width);
        self.canvas.set_height(height);
    }

    pub fn swap_buffers(&self) {
        // No need to do anything, the browser handles this.
    }

    pub fn glow(&self) -> Result<Arc<glow::Context>, ContextError> {
        Ok(self.gl.clone())
    }
}
