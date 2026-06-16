use std::fs;
use std::path::Path;

const SETTINGS_FILE: &str = "ndi-discovery-server.txt";
const NDI_CONFIG_FILE: &str = "ndi-config.v1.json";

pub fn apply_from_app_config(app_config_dir: &Path) -> Result<(), String> {
    let discovery = read_discovery_server(app_config_dir);
    apply_ndi_config(
        app_config_dir,
        discovery.as_deref().map(str::trim).filter(|value| !value.is_empty()),
    )
}

pub fn get_discovery_server(app_config_dir: &Path) -> Option<String> {
    read_discovery_server(app_config_dir)
}

pub fn set_discovery_server(
    app_config_dir: &Path,
    discovery_server: Option<String>,
) -> Result<(), String> {
    fs::create_dir_all(app_config_dir).map_err(|error| error.to_string())?;

    let path = app_config_dir.join(SETTINGS_FILE);
    let normalized = discovery_server
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    match normalized.as_ref() {
        Some(value) => fs::write(&path, value).map_err(|error| error.to_string())?,
        None => {
            let _ = fs::remove_file(&path);
        }
    }

    apply_ndi_config(app_config_dir, normalized.as_deref())
}

fn read_discovery_server(app_config_dir: &Path) -> Option<String> {
    let path = app_config_dir.join(SETTINGS_FILE);
    let content = fs::read_to_string(path).ok()?;
    let trimmed = content.trim().to_string();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn apply_ndi_config(app_config_dir: &Path, discovery_server: Option<&str>) -> Result<(), String> {
    let ndi_dir = app_config_dir.join("ndi");
    fs::create_dir_all(&ndi_dir).map_err(|error| error.to_string())?;

    let config_path = ndi_dir.join(NDI_CONFIG_FILE);

    if let Some(server) = discovery_server {
        let config = serde_json::json!({
            "ndi": {
                "networks": {
                    "ips": "",
                    "discovery": server
                }
            }
        });

        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;

        // SAFETY: NDI reads this before NDIlib_initialize on first use.
        unsafe {
            std::env::set_var("NDI_CONFIG_DIR", &ndi_dir);
        }
    } else {
        let _ = fs::remove_file(&config_path);
        unsafe {
            std::env::remove_var("NDI_CONFIG_DIR");
        }
    }

    Ok(())
}
