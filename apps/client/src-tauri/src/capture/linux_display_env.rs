use std::env;
use std::path::PathBuf;

/// Mirror `scripts/linux/remote-dev.sh` so xcap monitor enumeration works under Wayland/Xwayland.
pub fn ensure() {
    if env::var_os("DISPLAY").is_none() {
        env::set_var("DISPLAY", ":0");
    }

    if env::var_os("XAUTHORITY").is_some() {
        return;
    }

    if let Some(path) = find_xauthority() {
        tracing::debug!(path = %path.display(), "setting XAUTHORITY for Xwayland");
        env::set_var("XAUTHORITY", path);
    }
}

fn find_xauthority() -> Option<PathBuf> {
    if let Ok(runtime) = env::var("XDG_RUNTIME_DIR") {
        let runtime_dir = PathBuf::from(runtime);
        if let Ok(entries) = std::fs::read_dir(&runtime_dir) {
            let mut candidates: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(".mutter-Xwaylandauth."))
                })
                .collect();
            candidates.sort();
            if let Some(path) = candidates.into_iter().next() {
                return Some(path);
            }
        }
    }

    let home_auth = env::var_os("HOME").map(|home| PathBuf::from(home).join(".Xauthority"));
    if let Some(path) = home_auth {
        if path.is_file() {
            return Some(path);
        }
    }

    None
}
