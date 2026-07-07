#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("portal-capture-probe: Linux only");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use tandem_client_lib::capture::FrameSlot;
    use tandem_client_lib::capture::linux_screen::{
        is_wayland_session, LinuxPortalContext, LinuxScreenCaptureSession,
    };
    use tandem_client_lib::capture::find_source;

    tandem_client_lib::capture::ensure_linux_display_env();

    if !is_wayland_session() {
        eprintln!("portal-capture-probe: requires a Wayland session");
        std::process::exit(1);
    }

    let source = find_source("screen:portal").expect("find portal screen source");

    eprintln!("portal-capture-probe: starting portal capture (pick a screen in the dialog)");

    let frame_slot = Arc::new(FrameSlot::default());
    let portal = LinuxPortalContext {
        window_identifier: None,
    };

    let session = LinuxScreenCaptureSession::start(&source, frame_slot.clone(), portal)
        .unwrap_or_else(|error| {
            eprintln!("portal-capture-probe: FAIL start capture: {error}");
            std::process::exit(1);
        });

    thread::sleep(Duration::from_secs(3));
    let stats = frame_slot.stats();
    session.stop();

    if stats.published == 0 {
        eprintln!(
            "portal-capture-probe: FAIL no frames published (approve the portal dialog if shown)"
        );
        std::process::exit(1);
    }

    eprintln!(
        "portal-capture-probe: PASS published={} size={}x{} bytes={}",
        stats.published, stats.latest_width, stats.latest_height, stats.latest_bytes
    );
}
