use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::capture::PresentationWindow;

pub const EXTENSION_UUID: &str = "window-targeting@casa.tafu.tandem";
const LEGACY_EXTENSION_UUID: &str = "casa.tafu.tandem.window-targeting";

static INSTALL_ATTEMPTED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GnomeExtensionStatus {
    pub applicable: bool,
    pub installed: bool,
    pub enabled: bool,
    pub active: bool,
    pub needs_logout: bool,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionWindow {
    id: String,
    title: String,
    app: String,
}

#[zbus::proxy(
    interface = "casa.tafu.tandem.WindowTargeting",
    default_service = "casa.tafu.tandem.WindowTargeting",
    default_path = "/casa/tafu/tandem/WindowTargeting"
)]
trait WindowTargeting {
    fn ping(&self) -> zbus::Result<bool>;

    fn list_windows(&self) -> zbus::Result<String>;

    fn activate_window(&self, id: &str) -> zbus::Result<bool>;
}

pub fn is_applicable() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    let session_type = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if session_type != "wayland" {
        return false;
    }

    is_gnome_desktop()
}

fn is_gnome_desktop() -> bool {
    for key in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        let value = std::env::var(key).unwrap_or_default().to_ascii_uppercase();
        if value.contains("GNOME") || value.contains("UBUNTU") {
            return true;
        }
    }

    Path::new("/usr/bin/gnome-shell").exists()
}

#[zbus::proxy(
    interface = "org.gnome.Shell.Extensions",
    default_service = "org.gnome.Shell",
    default_path = "/org/gnome/Shell/Extensions"
)]
trait ShellExtensions {
    fn enable_extension(&self, uuid: &str) -> zbus::Result<bool>;
}

pub fn ensure_installed(handle: Option<&tauri::AppHandle>) {
    if !is_applicable() {
        return;
    }

    let _ = INSTALL_ATTEMPTED.get_or_init(|| {
        if let Err(error) = install_extension(handle) {
            tracing::warn!(%error, "failed to install GNOME Shell extension");
        }
    });
}

pub fn status(handle: Option<&tauri::AppHandle>) -> GnomeExtensionStatus {
    if !is_applicable() {
        return GnomeExtensionStatus {
            applicable: false,
            installed: false,
            enabled: false,
            active: false,
            needs_logout: false,
            message: None,
        };
    }

    ensure_installed(handle);

    let installed = extension_install_dir().map(|dir| dir.join("metadata.json").is_file()).unwrap_or(false);
    let enabled = extension_enabled();
    let active = dbus_ping();

    let needs_logout = installed && !active;
    let message = if needs_logout {
        Some(
            "Log out and sign back in once so Tandem can find browser and app windows for the remote clicker."
                .into(),
        )
    } else if !installed {
        Some("Tandem could not finish setting up the remote clicker.".into())
    } else {
        None
    };

    GnomeExtensionStatus {
        applicable: true,
        installed,
        enabled,
        active,
        needs_logout,
        message,
    }
}

pub fn dbus_ping() -> bool {
    with_proxy(|proxy| proxy.ping()).unwrap_or(false)
}

pub fn list_windows() -> Result<Vec<PresentationWindow>, String> {
    let raw = with_proxy(|proxy| proxy.list_windows()).map_err(|error| error.to_string())?;

    let entries: Vec<ExtensionWindow> =
        serde_json::from_str(&raw).map_err(|error| format!("Invalid window list from extension: {error}"))?;

    let mut windows = Vec::with_capacity(entries.len());
    for entry in entries {
        let label = if entry.app.trim().is_empty() {
            entry.title
        } else {
            format!("{} — {}", entry.title, entry.app)
        };

        windows.push(PresentationWindow {
            id: format!("window:{}", entry.id),
            label,
        });
    }

    windows.sort_by(|left, right| left.label.to_ascii_lowercase().cmp(&right.label.to_ascii_lowercase()));
    Ok(windows)
}

pub fn activate_window(window_id: u32) -> Result<(), String> {
    let success = with_proxy(|proxy| proxy.activate_window(&window_id.to_string()))
        .map_err(|error| error.to_string())?;

    if success {
        tracing::debug!(window_id, "activated presentation window via GNOME extension");
        Ok(())
    } else {
        Err(format!("Window {window_id} was not found"))
    }
}

fn with_proxy<T>(operation: impl FnOnce(&WindowTargetingProxyBlocking<'_>) -> zbus::Result<T>) -> zbus::Result<T> {
    let connection = zbus::blocking::Connection::session()?;
    let proxy = WindowTargetingProxyBlocking::new(&connection)?;
    operation(&proxy)
}

fn install_extension(handle: Option<&tauri::AppHandle>) -> Result<(), String> {
    let Some(source) = resolve_bundled_extension_source(handle) else {
        return Err("Bundled GNOME Shell extension files were not found".into());
    };

    let dest = extension_install_dir().ok_or_else(|| "Could not resolve home directory".to_string())?;
    remove_legacy_extension_dir();
    copy_dir_recursive(&source, &dest)?;

    if let Err(error) = enable_extension() {
        tracing::warn!(%error, "extension files installed; enable after logout/login");
    } else {
        tracing::info!(uuid = EXTENSION_UUID, path = %dest.display(), "installed GNOME Shell extension");
    }

    Ok(())
}

fn extension_install_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(".local")
            .join("share")
            .join("gnome-shell")
            .join("extensions")
            .join(EXTENSION_UUID)
    })
}

fn remove_legacy_extension_dir() {
    let Some(home) = dirs::home_dir() else {
        return;
    };

    let legacy = home
        .join(".local")
        .join("share")
        .join("gnome-shell")
        .join("extensions")
        .join(LEGACY_EXTENSION_UUID);

    if legacy.is_dir() {
        let _ = fs::remove_dir_all(&legacy);
    }
}

fn extension_enabled() -> bool {
    let output = Command::new(gnome_extensions_bin())
        .args(["list", "--enabled"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).lines().any(|line| line.trim() == EXTENSION_UUID)
        }
        _ => false,
    }
}

fn enable_extension() -> Result<(), String> {
    if enable_extension_via_cli() || enable_extension_via_shell() {
        return Ok(());
    }

    Err("Could not enable GNOME Shell extension — log out and back in, then reopen Tandem".into())
}

fn enable_extension_via_cli() -> bool {
    let output = Command::new(gnome_extensions_bin())
        .args(["enable", EXTENSION_UUID])
        .output();

    matches!(output, Ok(output) if output.status.success())
}

fn enable_extension_via_shell() -> bool {
    let connection = match zbus::blocking::Connection::session() {
        Ok(connection) => connection,
        Err(_) => return false,
    };

    let proxy = match ShellExtensionsProxyBlocking::new(&connection) {
        Ok(proxy) => proxy,
        Err(_) => return false,
    };

    proxy.enable_extension(EXTENSION_UUID).unwrap_or(false)
}

fn gnome_extensions_bin() -> &'static str {
    if Path::new("/usr/bin/gnome-extensions").is_file() {
        "/usr/bin/gnome-extensions"
    } else {
        "gnome-extensions"
    }
}

fn resolve_bundled_extension_source(handle: Option<&tauri::AppHandle>) -> Option<PathBuf> {
    if let Some(app) = handle {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let bundled = resource_dir
                .join("gnome-shell-extension")
                .join(EXTENSION_UUID);
            if bundled.join("metadata.json").is_file() {
                return Some(bundled);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("gnome-shell-extension").join(EXTENSION_UUID),
                dir.join("../lib/tandem-client/gnome-shell-extension").join(EXTENSION_UUID),
                dir.join("../share/tandem-client/gnome-shell-extension").join(EXTENSION_UUID),
            ] {
                if candidate.join("metadata.json").is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../gnome-shell-extension")
        .join(EXTENSION_UUID);
    if dev.join("metadata.json").is_file() {
        return Some(dev);
    }

    None
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|error| error.to_string())?;

    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let src_path = entry.path();
        let dst_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}
