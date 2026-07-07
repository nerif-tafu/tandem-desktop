use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use nokhwa::utils::CameraInfo;

const USB_IDS_PATHS: &[&str] = &[
    "/usr/share/hwdata/usb.ids",
    "/usr/share/misc/usb.ids",
    "/var/lib/usbutils/usb.ids",
];

/// Build webcam source labels for Linux, using USB product names instead of
/// generic V4L2 names like "UVC Camera (046d:0823)".
pub fn label_cameras(cameras: &[CameraInfo]) -> Vec<(usize, String)> {
    let usb_names = load_usb_product_names();
    let mut entries: Vec<CameraEntry> = cameras
        .iter()
        .enumerate()
        .map(|(index, camera)| CameraEntry {
            index,
            usb_key: resolve_usb_key(camera, index),
            label: resolve_label(camera, &usb_names),
        })
        .collect();

    let mut seen_usb_keys = HashSet::new();
    entries.retain(|entry| seen_usb_keys.insert(entry.usb_key.clone()));

    entries.sort_by_key(|entry| entry.index);
    disambiguate_duplicate_labels(&mut entries);

    entries
        .into_iter()
        .map(|entry| (entry.index, entry.label))
        .collect()
}

struct CameraEntry {
    index: usize,
    usb_key: String,
    label: String,
}

fn resolve_usb_key(camera: &CameraInfo, index: usize) -> String {
    if let Some(path) = device_path_from_description(camera.description()) {
        if let Some(sysfs_video) = sysfs_video_node(&path) {
            if let Some(sysfs_usb) = usb_device_sysfs_path(&sysfs_video) {
                if let Some((vendor_id, product_id)) = usb_ids_for_sysfs_node(&sysfs_usb) {
                    return format!(
                        "{vendor_id:04x}:{product_id:04x}@{}",
                        sysfs_usb.display()
                    );
                }
            }
        }
    }

    if let Some((vendor_id, product_id)) = usb_ids_from_human_name(&camera.human_name()) {
        return format!("{vendor_id:04x}:{product_id:04x}");
    }

    format!("fallback:{index}")
}

fn resolve_label(camera: &CameraInfo, usb_names: &HashMap<(u16, u16), String>) -> String {
    let human_name = camera.human_name();

    if let Some(path) = device_path_from_description(camera.description()) {
        if let Some(sysfs_video) = sysfs_video_node(&path) {
            if let Some(sysfs_usb) = usb_device_sysfs_path(&sysfs_video) {
                if let Some((vendor_id, product_id)) = usb_ids_for_sysfs_node(&sysfs_usb) {
                    if let Some(name) =
                        lookup_usb_name(vendor_id, product_id, &sysfs_usb, usb_names)
                    {
                        return name;
                    }
                }
            }
        }
    }

    if let Some((vendor_id, product_id)) = usb_ids_from_human_name(&human_name) {
        if let Some(name) = usb_names.get(&(vendor_id, product_id)) {
            return name.clone();
        }
    }

    human_name
}

fn lookup_usb_name(
    vendor_id: u16,
    product_id: u16,
    sysfs_usb: &Path,
    usb_names: &HashMap<(u16, u16), String>,
) -> Option<String> {
    let manufacturer = read_sysfs_string(&sysfs_usb.join("manufacturer"));
    let product = read_sysfs_string(&sysfs_usb.join("product"));
    if let Some(name) = combine_manufacturer_product(manufacturer.as_deref(), product.as_deref()) {
        return Some(name);
    }

    usb_names.get(&(vendor_id, product_id)).cloned()
}

fn device_path_from_description(description: &str) -> Option<String> {
    description
        .rsplit('@')
        .next()
        .map(str::trim)
        .filter(|path| path.starts_with("/dev/"))
        .map(str::to_string)
}

fn sysfs_video_node(device_path: &str) -> Option<PathBuf> {
    let name = Path::new(device_path).file_name()?;
    Some(PathBuf::from("/sys/class/video4linux").join(name))
}

fn usb_device_sysfs_path(sysfs_video: &Path) -> Option<PathBuf> {
    let mut current = sysfs_video.join("device");
    for _ in 0..8 {
        if current.join("idVendor").is_file() && current.join("idProduct").is_file() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
    None
}

fn usb_ids_for_sysfs_node(sysfs_usb: &Path) -> Option<(u16, u16)> {
    let vendor = parse_hex_id(&fs::read_to_string(sysfs_usb.join("idVendor")).ok()?)?;
    let product = parse_hex_id(&fs::read_to_string(sysfs_usb.join("idProduct")).ok()?)?;
    Some((vendor, product))
}

fn usb_ids_from_human_name(human_name: &str) -> Option<(u16, u16)> {
    let (vendor, product) = parse_uvc_ids(human_name)?;
    let vendor_id = u16::from_str_radix(&vendor, 16).ok()?;
    let product_id = u16::from_str_radix(&product, 16).ok()?;
    Some((vendor_id, product_id))
}

fn parse_hex_id(raw: &str) -> Option<u16> {
    u16::from_str_radix(raw.trim(), 16).ok()
}

fn read_sysfs_string(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn combine_manufacturer_product(manufacturer: Option<&str>, product: Option<&str>) -> Option<String> {
    match (manufacturer, product) {
        (Some(manufacturer), Some(product)) => {
            let manufacturer = manufacturer.trim();
            let product = product.trim();
            if product.is_empty() {
                return None;
            }
            if manufacturer.is_empty() || product.contains(manufacturer) {
                Some(product.to_string())
            } else {
                Some(format!("{manufacturer} {product}"))
            }
        }
        (None, Some(product)) if !product.trim().is_empty() => Some(product.trim().to_string()),
        (Some(manufacturer), None) if !manufacturer.trim().is_empty() => {
            Some(manufacturer.trim().to_string())
        }
        _ => None,
    }
}

fn parse_uvc_ids(human_name: &str) -> Option<(String, String)> {
    let start = human_name.find('(')?;
    let end = human_name.find(')')?;
    let ids = &human_name[start + 1..end];
    let (vendor, product) = ids.split_once(':')?;
    if vendor.len() != 4 || product.len() != 4 {
        return None;
    }
    Some((vendor.to_string(), product.to_string()))
}

fn load_usb_product_names() -> HashMap<(u16, u16), String> {
    for path in USB_IDS_PATHS {
        if let Ok(content) = fs::read_to_string(path) {
            return parse_usb_ids(&content);
        }
    }
    HashMap::new()
}

fn parse_usb_ids(content: &str) -> HashMap<(u16, u16), String> {
    let mut names = HashMap::new();
    let mut current_vendor: Option<u16> = None;

    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        if line.starts_with('\t') {
            let Some(vendor) = current_vendor else {
                continue;
            };
            let trimmed = line.trim();
            let Some((id, name)) = trimmed.split_once(char::is_whitespace) else {
                continue;
            };
            let Ok(product) = u16::from_str_radix(id, 16) else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() {
                names.insert((vendor, product), name.to_string());
            }
            continue;
        }

        let Some((id, _name)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        current_vendor = u16::from_str_radix(id, 16).ok();
    }

    names
}

fn disambiguate_duplicate_labels(entries: &mut [CameraEntry]) {
    let mut label_counts = HashMap::<String, usize>::new();
    for entry in entries.iter() {
        *label_counts.entry(entry.label.clone()).or_default() += 1;
    }

    let mut label_indexes = HashMap::<String, usize>::new();
    for entry in entries.iter_mut() {
        let count = label_counts.get(&entry.label).copied().unwrap_or(1);
        if count <= 1 {
            continue;
        }

        let next = label_indexes.entry(entry.label.clone()).or_insert(0);
        *next += 1;
        entry.label = format!("{} #{}", entry.label, *next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_device_path_from_nokhwa_description() {
        assert_eq!(
            device_path_from_description("Video4Linux Device @ /dev/video0").as_deref(),
            Some("/dev/video0")
        );
    }

    #[test]
    fn parses_uvc_ids_from_human_name() {
        assert_eq!(
            parse_uvc_ids("UVC Camera (046d:0823)"),
            Some(("046d".to_string(), "0823".to_string()))
        );
        assert_eq!(
            usb_ids_from_human_name("UVC Camera (046d:0823)"),
            Some((0x046d, 0x0823))
        );
    }

    #[test]
    fn parses_usb_ids_product_entries() {
        let content = "046d  Logitech, Inc.\n\t0823  HD Webcam B910\n";
        let names = parse_usb_ids(content);
        assert_eq!(
            names.get(&(0x046d, 0x0823)),
            Some(&"HD Webcam B910".to_string())
        );
    }

    #[test]
    fn resolves_label_from_uvc_human_name() {
        let usb_names = parse_usb_ids("046d  Logitech, Inc.\n\t0823  HD Webcam B910\n");
        let camera = CameraInfo::new(
            "UVC Camera (046d:0823)",
            "Video4Linux Device @ /dev/video0",
            "",
            nokhwa::utils::CameraIndex::Index(0),
        );
        assert_eq!(resolve_label(&camera, &usb_names), "HD Webcam B910");
    }
}
