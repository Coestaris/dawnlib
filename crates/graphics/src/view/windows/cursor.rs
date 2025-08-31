use crate::view::windows::{get_last_error, ViewError};
use crate::view::{View, ViewCursor};
use log::debug;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    LoadCursorW, SetCursor, ShowCursor, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM,
    IDC_NO, IDC_SIZEALL, IDC_WAIT,
};

impl ViewCursor {
    fn to_win(&self) -> PCWSTR {
        match self {
            ViewCursor::Arrow => IDC_ARROW,
            ViewCursor::Default => IDC_ARROW,
            ViewCursor::Crosshair => IDC_CROSS,
            ViewCursor::Hand => IDC_HAND,
            ViewCursor::Move => IDC_SIZEALL,
            ViewCursor::Text => IDC_IBEAM,
            ViewCursor::Wait => IDC_WAIT,
            ViewCursor::Help => IDC_HELP,
            ViewCursor::NotAllowed => IDC_NO,
            _ => IDC_ARROW, // Fallback to arrow for unsupported cursors
        }
    }
}

impl View {
    pub(super) fn set_cursor_inner(&mut self, cursor: ViewCursor) -> Result<(), ViewError> {
        debug!("Setting window cursor: {:?}", cursor);
        unsafe {
            if matches!(cursor, ViewCursor::Hidden) {
                ShowCursor(false);
                Ok(())
            } else {
                ShowCursor(true);
                let hcursor = LoadCursorW(None, cursor.to_win())
                    .map_err(|_| ViewError::CreateCursorError(get_last_error()))?;

                SetCursor(Some(hcursor));
                self.cursor = cursor;
                Ok(())
            }
        }
    }
}
