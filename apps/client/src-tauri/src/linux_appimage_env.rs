/// AppImage GTK/WebKit defaults that avoid EGL crashes on VMs and mismatched GIO modules.
pub fn prepare() {
    if !running_inside_appimage() {
        return;
    }

    set_if_unset("GSK_RENDERER", "cairo");
    set_if_unset("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    set_if_unset("LIBGL_ALWAYS_SOFTWARE", "1");

    if let Some(gio_modules) = bundled_gio_module_dir() {
        std::env::set_var("GIO_MODULE_DIR", gio_modules);
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
