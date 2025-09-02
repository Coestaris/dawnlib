use crate::view::{View, ViewGeometry};
use glam::UVec2;
use log::debug;
use std::mem::size_of;
use windows::Win32::Graphics::Gdi::CDS_TYPE;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{
        ChangeDisplaySettingsW, EnumDisplaySettingsW, GetMonitorInfoW, MonitorFromWindow,
        CDS_FULLSCREEN, DEVMODEW, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
        ENUM_CURRENT_SETTINGS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    },
    UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
        GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_VISIBLE,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowMode {
    Normal,
    BorderlessFullscreen,
    FullscreenExclusive,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SavedWindowState {
    style: isize,
    ex_style: isize,
    rect: RECT,
    injected_topmost: bool,
}

impl View {
    pub(super) fn set_geometry_inner(
        &mut self,
        geometry: ViewGeometry,
    ) -> Result<(), crate::view::ViewError> {
        debug!(
            "Set geometry: {:?}, current mode: {:?}",
            geometry, self.mode
        );
        unsafe {
            match geometry {
                ViewGeometry::Normal(size) => {
                    if self.mode == WindowMode::Normal {
                        // Already normal - just resize
                        SetWindowPos(
                            self.hwnd,
                            None,
                            0,
                            0,
                            size.x as i32,
                            size.y as i32,
                            SWP_NOMOVE | SWP_NOZORDER,
                        )
                        .ok();
                        return Ok(());
                    }

                    // If was in fullscreen mode, restore saved state
                    match self.mode {
                        WindowMode::BorderlessFullscreen => self.leave_borderless_fullscreen(),
                        WindowMode::FullscreenExclusive => self.leave_exclusive_fullscreen(),
                        WindowMode::Normal => {}
                    }

                    SetWindowPos(
                        self.hwnd,
                        None,
                        0,
                        0,
                        size.x as i32,
                        size.y as i32,
                        SWP_NOMOVE | SWP_NOZORDER,
                    )
                    .ok();
                    self.mode = WindowMode::Normal;
                    Ok(())
                }

                ViewGeometry::BorderlessFullscreen => {
                    if self.mode == WindowMode::BorderlessFullscreen {
                        return Ok(());
                    }
                    self.ensure_saved_window_state();

                    if self.mode == WindowMode::FullscreenExclusive {
                        self.leave_exclusive_fullscreen();
                    }

                    self.enter_borderless_fullscreen();
                    self.mode = WindowMode::BorderlessFullscreen;
                    Ok(())
                }

                ViewGeometry::Fullscreen => {
                    if self.mode == WindowMode::FullscreenExclusive {
                        return Ok(());
                    }
                    self.ensure_saved_window_state();

                    if self.mode == WindowMode::BorderlessFullscreen {
                        self.leave_borderless_fullscreen();
                    }

                    self.enter_exclusive_fullscreen();
                    self.mode = WindowMode::FullscreenExclusive;
                    Ok(())
                }
            }
        }
    }

    unsafe fn ensure_saved_window_state(&mut self) {
        if self.mode == WindowMode::Normal && self.saved.is_none() {
            let style = GetWindowLongPtrW(self.hwnd, GWL_STYLE);
            let ex_style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            let mut rect = RECT::default();
            GetWindowRect(self.hwnd, &mut rect);
            self.saved = Some(SavedWindowState {
                style,
                ex_style,
                rect,
                injected_topmost: false,
            });
        }
    }

    unsafe fn monitor_rect_logical(&self) -> (RECT, i32) {
        let hmonitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(hmonitor, &mut mi).unwrap();
        let dpi = GetDpiForWindow(self.hwnd) as i32;
        (mi.rcMonitor, dpi)
    }

    unsafe fn logical_to_physical(&self, dpi: i32, rect: RECT) -> (i32, i32) {
        let w_log = rect.right - rect.left;
        let h_log = rect.bottom - rect.top;

        unsafe {
            let w_px = (w_log * dpi + 96 / 2) / 96;
            let h_px = (h_log * dpi + 96 / 2) / 96;
            (w_px, h_px)
        }
    }

    unsafe fn enter_borderless_fullscreen(&mut self) {
        debug!("Entering borderless fullscreen");

        let style_old = GetWindowLongPtrW(self.hwnd, GWL_STYLE) as u32;
        let new_style = (style_old & !WS_OVERLAPPEDWINDOW.0) | WS_POPUP.0 | WS_VISIBLE.0;
        SetWindowLongPtrW(self.hwnd, GWL_STYLE, new_style as isize);

        let (monitor_rect, dpi) = self.monitor_rect_logical();
        debug!("Logical monitor rect: {:?}. DPI: {}", monitor_rect, dpi);
        let (w, h) = self.logical_to_physical(dpi, monitor_rect);
        debug!("Physical size: {}x{}", w, h);

        SetWindowPos(
            self.hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            w,
            h,
            SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
        )
        .ok();

        self.events_sender
            .send(crate::view::InputEvent::Resize(UVec2::new(
                w as u32, h as u32,
            )))
            .ok();

        ShowWindow(self.hwnd, SW_SHOW);

        if let Some(ref mut s) = self.saved {
            s.injected_topmost = true;
        }
    }

    unsafe fn leave_borderless_fullscreen(&mut self) {
        debug!("Leaving borderless fullscreen");

        if let Some(saved) = self.saved {
            SetWindowLongPtrW(self.hwnd, GWL_STYLE, saved.style);
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, saved.ex_style);

            let w = saved.rect.right - saved.rect.left;
            let h = saved.rect.bottom - saved.rect.top;

            SetWindowPos(
                self.hwnd,
                if saved.injected_topmost {
                    Some(HWND_NOTOPMOST)
                } else {
                    None
                },
                saved.rect.left,
                saved.rect.top,
                w,
                h,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
            )
            .ok();

            self.events_sender
                .send(crate::view::InputEvent::Resize(UVec2::new(
                    w as u32, h as u32,
                )))
                .ok();

            ShowWindow(self.hwnd, SW_SHOW);
        }
        self.mode = WindowMode::Normal;
    }

    unsafe fn enter_exclusive_fullscreen(&mut self) {
        debug!("Entering exclusive fullscreen");

        let mut dm = DEVMODEW {
            dmSize: size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        if EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut dm).into() {
            let (rect, dpi) = self.monitor_rect_logical();
            debug!("Logical monitor rect: {:?}. DPI: {}", rect, dpi);
            let (w, h) = self.logical_to_physical(dpi, rect);
            debug!("Physical size: {}x{}", w, h);

            dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;

            ChangeDisplaySettingsW(Some(&dm), CDS_FULLSCREEN);

            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                w,
                h,
                SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
            )
            .ok();

            self.events_sender
                .send(crate::view::InputEvent::Resize(UVec2::new(
                    w as u32, h as u32,
                )))
                .ok();

            ShowWindow(self.hwnd, SW_SHOW);

            if let Some(ref mut s) = self.saved {
                s.injected_topmost = true;
            }
        }
    }

    unsafe fn leave_exclusive_fullscreen(&mut self) {
        debug!("Leaving exclusive fullscreen");

        ChangeDisplaySettingsW(None, CDS_TYPE(0));

        if let Some(saved) = self.saved {
            SetWindowLongPtrW(self.hwnd, GWL_STYLE, saved.style);
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, saved.ex_style);

            let w = saved.rect.right - saved.rect.left;
            let h = saved.rect.bottom - saved.rect.top;

            SetWindowPos(
                self.hwnd,
                if saved.injected_topmost {
                    Some(HWND_NOTOPMOST)
                } else {
                    None
                },
                saved.rect.left,
                saved.rect.top,
                w,
                h,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOOWNERZORDER,
            )
            .ok();

            self.events_sender
                .send(crate::view::InputEvent::Resize(UVec2::new(
                    w as u32, h as u32,
                )))
                .ok();

            ShowWindow(self.hwnd, SW_SHOW);
        }

        self.mode = WindowMode::Normal;
    }
}
