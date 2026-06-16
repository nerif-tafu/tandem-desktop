use std::path::PathBuf;

use tauri::image::Image;
use tauri::{AppHandle, Manager};

pub fn apply_window_icons(app: &AppHandle) -> Result<(), String> {
    let icon = load_window_icon(app)?;

    for (_, window) in app.webview_windows() {
        window
            .set_icon(icon.clone())
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn load_window_icon(_app: &AppHandle) -> Result<Image<'static>, String> {
    if cfg!(debug_assertions) {
        let dev_icon = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png");
        if dev_icon.exists() {
            if let Ok(icon) = Image::from_path(&dev_icon) {
                return Ok(icon.to_owned());
            }
        }
    }

    Image::from_bytes(include_bytes!("../icons/32x32.png"))
        .map(|icon| icon.to_owned())
        .map_err(|error| error.to_string())
}
