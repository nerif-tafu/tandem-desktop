mod capture;
mod logging;
#[cfg(target_os = "linux")]
pub mod linux_appimage_env;
mod ndi_config;
mod presentation;
mod window_icon;

use std::sync::Mutex;

use capture::{
    list_all_sources, CaptureDiagnostics, CaptureManager, CaptureSource, PresentationWindow,
    SlotCaptureState, VideoCaptureManager, STREAM_SLOTS, slot_label,
};
use presentation::KeyboardPresentationController;
use tauri::Manager;

#[cfg(all(windows, feature = "ndi"))]
fn stage_ndi_runtime(app: &tauri::App) -> Result<(), String> {
    stage_ndi_runtime_file(
        app,
        &["Processing.NDI.Lib.x64.dll"],
        &["Processing.NDI.Lib.x64.dll"],
    )
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn stage_ndi_runtime(app: &tauri::App) -> Result<(), String> {
    stage_ndi_runtime_file(
        app,
        &["libndi.dylib", "libndi.4.dylib"],
        &["libndi.dylib", "libndi.4.dylib"],
    )
}

#[cfg(feature = "ndi")]
fn stage_ndi_runtime_file(
    app: &tauri::App,
    file_names: &[&str],
    resource_names: &[&str],
) -> Result<(), String> {
    use std::fs;

    let Ok(exe_dir) = app.path().executable_dir() else {
        return Ok(());
    };

    if file_names.iter().any(|name| exe_dir.join(name).exists()) {
        return Ok(());
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        for resource_name in resource_names {
            for bundled in [
                resource_dir.join(resource_name),
                resource_dir.join("ndi").join(resource_name),
            ] {
                if bundled.exists() {
                    let dest = exe_dir.join(resource_name);
                    let _ = fs::copy(&bundled, &dest);
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            is_dev_mode,
            is_ndi_available,
            get_ndi_discovery_server,
            set_ndi_discovery_server,
            presentation_forward,
            presentation_back,
            set_presentation_target,
            get_presentation_target,
            list_capture_sources,
            list_presentation_windows,
            set_slot_source,
            get_slot_states,
            refresh_slot_preview,
            start_slot_video,
            stop_slot_video,
            get_video_server_port,
            append_client_log,
            get_client_log_path,
            get_capture_diagnostics,
        ])
        .setup(|app| {
            if let Ok(log_dir) = app.path().app_log_dir() {
                logging::init(&log_dir);
            } else {
                tracing_subscriber::fmt()
                    .with_env_filter("info")
                    .init();
                tracing::warn!("could not resolve app log directory; logging to stdout only");
            }

            #[cfg(windows)]
            capture::disable_background_throttling();

            if let Ok(config_dir) = app.path().app_config_dir() {
                ndi_config::apply_from_app_config(&config_dir)?;
            }

            #[cfg(feature = "ndi")]
            stage_ndi_runtime(app)?;

            app.manage(KeyboardPresentationController::new());
            app.manage(Mutex::new(CaptureManager::new()));
            app.manage(
                Mutex::new(
                    VideoCaptureManager::new()
                        .expect("failed to start local video stream server"),
                ),
            );

            if let Err(error) = window_icon::apply_window_icons(app.handle()) {
                tracing::warn!(%error, "failed to apply window icon");
            }

            capture::spawn_diagnostics_task(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tandem client");
}

#[tauri::command]
fn is_dev_mode() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn is_ndi_available() -> bool {
    capture::ndi_is_available()
}

#[tauri::command]
fn get_ndi_discovery_server(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let config_dir = app.path().app_config_dir().map_err(|error| error.to_string())?;
    Ok(ndi_config::get_discovery_server(&config_dir))
}

#[tauri::command]
fn set_ndi_discovery_server(
    app: tauri::AppHandle,
    discovery_server: Option<String>,
) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|error| error.to_string())?;
    ndi_config::set_discovery_server(&config_dir, discovery_server)
}

#[tauri::command]
fn presentation_forward(
    controller: tauri::State<'_, KeyboardPresentationController>,
) -> Result<(), String> {
    controller.forward().map_err(|error| error.to_string())
}

#[tauri::command]
fn presentation_back(
    controller: tauri::State<'_, KeyboardPresentationController>,
) -> Result<(), String> {
    controller.back().map_err(|error| error.to_string())
}

#[tauri::command]
fn set_presentation_target(
    source_id: Option<String>,
    controller: tauri::State<'_, KeyboardPresentationController>,
) -> Result<(), String> {
    controller
        .set_target(source_id.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_presentation_target(
    controller: tauri::State<'_, KeyboardPresentationController>,
) -> Result<Option<String>, String> {
    controller.get_target().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_capture_sources() -> Result<Vec<CaptureSource>, String> {
    list_all_sources().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_presentation_windows() -> Result<Vec<PresentationWindow>, String> {
    capture::list_presentation_windows().map_err(|error| error.to_string())
}

#[tauri::command]
fn set_slot_source(
    slot: String,
    source_id: Option<String>,
    manager: tauri::State<'_, Mutex<CaptureManager>>,
    video: tauri::State<'_, Mutex<VideoCaptureManager>>,
) -> Result<SlotCaptureState, String> {
    if !STREAM_SLOTS.contains(&slot.as_str()) {
        return Err(format!("Unknown slot: {slot}"));
    }

    let resolved_source = if let Some(ref id) = source_id {
        Some(capture::find_source(id).map_err(|error| error.to_string())?)
    } else {
        None
    };

    let mut guard = manager
        .lock()
        .map_err(|_| "Capture manager unavailable".to_string())?;

    let previous_id = guard.get_assignment(&slot).and_then(|id| id.clone());
    let next_id = source_id.clone();

    guard.set_assignment(&slot, source_id);

    let source_changed = previous_id.as_deref() != next_id.as_deref();
    if resolved_source.is_none() || source_changed {
        let video_guard = video
            .lock()
            .map_err(|_| "Video capture manager unavailable".to_string())?;
        video_guard.stop_slot(&slot);
    }

    build_slot_state(&slot, &guard, resolved_source.as_ref())
}

#[tauri::command]
fn get_slot_states(
    manager: tauri::State<'_, Mutex<CaptureManager>>,
) -> Result<Vec<SlotCaptureState>, String> {
    let guard = manager
        .lock()
        .map_err(|_| "Capture manager unavailable".to_string())?;

    STREAM_SLOTS
        .iter()
        .map(|slot| {
            let source_id = guard.get_assignment(slot).and_then(|id| id.clone());
            let resolved_source = match source_id.as_deref() {
                Some(id) => Some(capture::find_source(id).map_err(|error| error.to_string())?),
                None => None,
            };

            build_slot_state(slot, &guard, resolved_source.as_ref())
        })
        .collect()
}

#[tauri::command]
fn refresh_slot_preview(
    slot: String,
    manager: tauri::State<'_, Mutex<CaptureManager>>,
) -> Result<SlotCaptureState, String> {
    if !STREAM_SLOTS.contains(&slot.as_str()) {
        return Err(format!("Unknown slot: {slot}"));
    }

    let guard = manager
        .lock()
        .map_err(|_| "Capture manager unavailable".to_string())?;

    let source_id = guard.get_assignment(&slot).and_then(|id| id.clone());
    let resolved_source = match source_id.as_deref() {
        Some(id) => Some(capture::find_source(id).map_err(|error| error.to_string())?),
        None => None,
    };

    build_slot_state(&slot, &guard, resolved_source.as_ref())
}

#[tauri::command]
fn start_slot_video(
    slot: String,
    source_id: String,
    video: tauri::State<'_, Mutex<VideoCaptureManager>>,
) -> Result<String, String> {
    if !STREAM_SLOTS.contains(&slot.as_str()) {
        return Err(format!("Unknown slot: {slot}"));
    }

    let source = capture::find_source(&source_id).map_err(|error| error.to_string())?;
    let guard = video
        .lock()
        .map_err(|_| "Video capture manager unavailable".to_string())?;

    guard.start_slot(&slot, &source)
}

#[tauri::command]
fn stop_slot_video(
    slot: String,
    video: tauri::State<'_, Mutex<VideoCaptureManager>>,
) -> Result<(), String> {
    let guard = video
        .lock()
        .map_err(|_| "Video capture manager unavailable".to_string())?;

    guard.stop_slot(&slot);
    Ok(())
}

#[tauri::command]
fn get_video_server_port(
    video: tauri::State<'_, Mutex<VideoCaptureManager>>,
) -> Result<u16, String> {
    let guard = video
        .lock()
        .map_err(|_| "Video capture manager unavailable".to_string())?;

    Ok(guard.server_port())
}

#[tauri::command]
fn append_client_log(line: String) -> Result<(), String> {
    logging::append_client_log(&line)
}

#[tauri::command]
fn get_client_log_path() -> Result<Option<String>, String> {
    Ok(logging::log_path_hint())
}

#[tauri::command]
fn get_capture_diagnostics(
    video: tauri::State<'_, Mutex<VideoCaptureManager>>,
) -> Result<capture::CaptureDiagnostics, String> {
    let guard = video
        .lock()
        .map_err(|_| "Video capture manager unavailable".to_string())?;

    Ok(guard.diagnostics())
}

fn build_slot_state(
    slot: &str,
    _manager: &CaptureManager,
    source: Option<&CaptureSource>,
) -> Result<SlotCaptureState, String> {
    let source = source.cloned();

    Ok(SlotCaptureState {
        slot: slot.to_string(),
        label: slot_label(slot).to_string(),
        active: source.is_some(),
        source,
        preview: None,
    })
}
