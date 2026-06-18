/// Linux AppImage defaults for GTK/WebKit graphics compatibility.
/// LD_PRELOAD for host libwayland-client is applied in `scripts/linux/patch-appimage.sh`
/// at build time (must run before the binary loads bundled libs).
/// See https://v2.tauri.app/develop/debug/linux-graphics/
pub fn prepare() {
    if !running_inside_appimage() {
        return;
    }

    set_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    set_if_unset("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    if let Some(gio_modules) = bundled_gio_module_dir() {
        std::env::set_var("GIO_MODULE_DIR", gio_modules);
    }

    if let Some(exe_dir) = current_exe_dir() {
        prepend_path_var("LD_LIBRARY_PATH", &exe_dir);
    }
}

fn running_inside_appimage() -> bool {
    std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some()
}

fn bundled_gio_module_dir() -> Option<String> {
    let appdir = std::env::var_os("APPDIR")?;
    let path = std::path::Path::new(&appdir).join("usr/lib/x86_64-linux-gnu/gio/modules");
    path.is_dir().then(|| path.to_string_lossy().into_owned())
}

fn set_if_unset(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}

fn current_exe_dir() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    exe.parent()
        .map(|path| path.to_string_lossy().into_owned())
}

fn prepend_path_var(key: &str, entry: &str) {
    let next = match std::env::var_os(key) {
        Some(existing) => {
            let mut combined = entry.to_owned();
            combined.push(':');
            combined.push_str(&existing.to_string_lossy());
            combined
        }
        None => entry.to_owned(),
    };

    std::env::set_var(key, next);
}
