use crate::input::{InputEvent, MouseButton};
use crate::view::windows::input::convert_key;
use crate::view::{TickResult, View};
use log::debug;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_APP_QUIT_REQUESTED: u32 = WM_APP + 1;
pub const WM_APP_RESIZED: u32 = WM_APP + 2;

impl View {
    pub(super) fn tick_inner(&mut self) -> TickResult {
        let mut closed = false;
        let mut msg = MSG::default();
        while unsafe { GetMessageW(&mut msg, Some(self.hwnd), 0, 0).0 != 0 } {
            unsafe {
                DispatchMessageW(&msg);
            }

            /* Process the message synchronously
             * to make things simpler */
            let event: InputEvent;
            match msg.message {
                WM_APP_QUIT_REQUESTED => {
                    debug!("WM_APP_QUIT_REQUESTED received, closing the window");
                    closed = true;
                    continue;
                }
                WM_KEYDOWN => {
                    event = InputEvent::KeyPress(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_KEYUP => {
                    event = InputEvent::KeyRelease(convert_key(VIRTUAL_KEY(msg.wParam.0 as u16)));
                }
                WM_LBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Left);
                }
                WM_LBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(MouseButton::Left);
                }
                WM_MBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Middle);
                }
                WM_MBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(MouseButton::Middle);
                }
                WM_MOUSEMOVE => {
                    let x = (msg.lParam.0 as i32 & 0xFFFF) as f32;
                    let y = (msg.lParam.0 >> 16) as i32 as f32;
                    event = InputEvent::MouseMove { x, y };
                }
                WM_MOUSEWHEEL => {
                    let delta = (msg.wParam.0 as i32 >> 16) as f32 / 120.0; // Convert to standard scroll units
                    event = InputEvent::MouseScroll {
                        delta_x: 0.0,
                        delta_y: delta,
                    };
                }
                WM_APP_RESIZED => {
                    let width = msg.wParam.0 as u32;
                    let height = msg.lParam.0 as u32;
                    event = InputEvent::Resize {
                        width: width as usize,
                        height: height as usize,
                    };
                }
                WM_RBUTTONDOWN => {
                    event = InputEvent::MouseButtonPress(MouseButton::Right);
                }
                WM_RBUTTONUP => {
                    event = InputEvent::MouseButtonRelease(MouseButton::Right);
                }
                _ => {
                    return if !closed {
                        TickResult::Continue
                    } else {
                        TickResult::Closed
                    }
                }
            }

            self.events_sender.send(event).unwrap();
        }

        if !closed {
            TickResult::Continue
        } else {
            TickResult::Closed
        }
    }
}

pub(super) unsafe extern "system" fn win_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => LRESULT(0),

        WM_SIZE => {
            let width = (lparam.0 & 0xFFFF) as u32;
            let height = (lparam.0 >> 16) as u32;
            let _ = PostMessageW(
                Some(hwnd),
                WM_APP_RESIZED,
                WPARAM(width as usize),
                LPARAM(height as isize),
            );
            LRESULT(0)
        }

        WM_SIZING => LRESULT(1),

        WM_ENTERSIZEMOVE | WM_EXITSIZEMOVE => {
            let rect = RECT::default();
            unsafe {
                GetClientRect(hwnd, &rect as *const RECT as *mut RECT);
            }
            let width = (rect.right - rect.left) as u32;
            let height = (rect.bottom - rect.top) as u32;
            let _ = PostMessageW(
                Some(hwnd),
                WM_APP_RESIZED,
                WPARAM(width as usize),
                LPARAM(height as isize),
            );

            LRESULT(0)
        }

        WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED => unsafe {
            DefWindowProcW(hwnd, message, wparam, lparam)
        },

        WM_CLOSE => {
            /* Send a custom message to request the application to quit */
            let _ = PostMessageW(Some(hwnd), WM_APP_QUIT_REQUESTED, WPARAM(0), LPARAM(0));
            /* Block the message loop until the window is destroyed */
            LRESULT(0)
        }

        WM_DESTROY => {
            debug!("WM_DESTROY received, destroying window");
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }

        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
