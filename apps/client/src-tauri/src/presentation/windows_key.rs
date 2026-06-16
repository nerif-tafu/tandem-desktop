use super::PresentationError;
use enigo::Key;

pub fn post_presentation_key(window_id: u32, key: Key) -> Result<(), PresentationError> {
    use std::ffi::c_void;

    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LEFT, VK_NEXT, VK_PRIOR, VK_RIGHT};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP};

    if window_id == 0 {
        return Err(PresentationError::Input("Window is no longer available".into()));
    }

    let (arrow_vk, page_vk) = match key {
        Key::RightArrow => (VK_RIGHT, VK_NEXT),
        Key::LeftArrow => (VK_LEFT, VK_PRIOR),
        _ => {
            return Err(PresentationError::Input(format!(
                "Unsupported presentation key: {key:?}"
            )));
        }
    };

    unsafe {
        let hwnd = HWND(window_id as *mut c_void);
        post_vk(hwnd, arrow_vk)?;
        post_vk(hwnd, page_vk)?;
    }

    tracing::info!(?key, window_id, "posted presentation key to target window");
    Ok(())
}

unsafe fn post_vk(
    hwnd: windows::Win32::Foundation::HWND,
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
) -> Result<(), PresentationError> {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_KEYDOWN, WM_KEYUP};

    PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(vk.0 as usize), LPARAM(0))
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    PostMessageW(Some(hwnd), WM_KEYUP, WPARAM(vk.0 as usize), LPARAM(0))
        .map_err(|error| PresentationError::Input(error.to_string()))?;

    Ok(())
}
