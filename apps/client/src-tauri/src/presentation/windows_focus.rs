use super::PresentationError;

pub fn focus_window(window_id: u32) -> Result<(), PresentationError> {
    use std::ffi::c_void;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, BringWindowToTop, IsIconic, SetForegroundWindow, ShowWindow,
        ASFW_ANY, SW_RESTORE,
    };

    if window_id == 0 {
        return Err(PresentationError::Input("Window is no longer available".into()));
    }

    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let hwnd = HWND(window_id as *mut c_void);

        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let _ = BringWindowToTop(hwnd);
        if !SetForegroundWindow(hwnd).as_bool() {
            tracing::warn!(window_id, "could not bring presentation window to foreground");
        } else {
            tracing::debug!(window_id, "focused presentation window");
        }
    }

    Ok(())
}
