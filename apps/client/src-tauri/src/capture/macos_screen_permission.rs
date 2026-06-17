#[cfg(target_os = "macos")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

use std::sync::atomic::{AtomicBool, Ordering};

static PROMPTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
pub fn has_access() -> bool {
    // SAFETY: CoreGraphics screen-capture preflight is documented for macOS 10.15+.
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(target_os = "macos")]
pub fn request_access() -> bool {
    // SAFETY: CoreGraphics screen-capture request may open the system permission UI.
    unsafe { CGRequestScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
pub fn has_access() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_access() -> bool {
    true
}

pub fn access_required() -> bool {
    cfg!(target_os = "macos")
}

pub fn ensure_access() {
    if !access_required() || has_access() {
        return;
    }

    if PROMPTED.swap(true, Ordering::Relaxed) {
        return;
    }

    request_access();
}
