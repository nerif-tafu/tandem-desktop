use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    for icon in ["icon.ico", "icon.png", "32x32.png", "128x128.png", "128x128@2x.png"] {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join("icons").join(icon).display()
        );
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
    }

    copy_ndi_runtime(&manifest_dir);
    tauri_build::build();
}

fn copy_ndi_runtime(manifest_dir: &Path) {
    if cfg!(windows) {
        copy_ndi_runtime_windows(manifest_dir);
    }

    #[cfg(all(target_os = "macos", feature = "ndi"))]
    copy_ndi_runtime_macos(manifest_dir);
}

fn copy_ndi_runtime_windows(manifest_dir: &Path) {
    let dll_src = manifest_dir
        .join("..")
        .join("ndi-sdk")
        .join("Bin")
        .join("x64")
        .join("Processing.NDI.Lib.x64.dll");

    println!("cargo:rerun-if-changed={}", dll_src.display());

    if !dll_src.exists() {
        println!(
            "cargo:warning=NDI runtime DLL missing at {} — NDI capture will not work until it is present",
            dll_src.display()
        );
        return;
    }

    let profile = env::var("PROFILE").expect("PROFILE");
    let target_dir = manifest_dir.join("target").join(profile);
    let dll_dst = target_dir.join("Processing.NDI.Lib.x64.dll");

    if fs::copy(&dll_src, &dll_dst).is_err() {
        println!(
            "cargo:warning=Failed to copy NDI runtime DLL to {}",
            dll_dst.display()
        );
    }
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn copy_ndi_runtime_macos(manifest_dir: &Path) {
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");

    let Some(dylib_src) = resolve_macos_ndi_dylib() else {
        println!(
            "cargo:warning=NDI runtime dylib not found — install the NDI SDK for Apple or set NDI_SDK_DIR"
        );
        return;
    };

    println!("cargo:rerun-if-changed={}", dylib_src.display());

    let profile = env::var("PROFILE").expect("PROFILE");
    let target_dir = manifest_dir.join("target").join(profile);
    let runtime_dir = manifest_dir.join("ndi-runtime");
    if fs::create_dir_all(&runtime_dir).is_err() {
        println!(
            "cargo:warning=Failed to create NDI runtime directory at {}",
            runtime_dir.display()
        );
        return;
    }

    for dest_dir in [&runtime_dir, &target_dir] {
        copy_macos_ndi_dylib(&dylib_src, dest_dir);
    }
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn copy_macos_ndi_dylib(dylib_src: &Path, dest_dir: &Path) {
    let file_name = dylib_src
        .file_name()
        .expect("NDI dylib path should include a file name");

    let primary_dest = dest_dir.join(file_name);
    if fs::copy(dylib_src, &primary_dest).is_err() {
        println!(
            "cargo:warning=Failed to copy NDI runtime dylib to {}",
            primary_dest.display()
        );
        return;
    }

    fix_dylib_install_name(&primary_dest);

    if file_name != "libndi.dylib" {
        let alias_dest = dest_dir.join("libndi.dylib");
        if fs::copy(dylib_src, &alias_dest).is_ok() {
            fix_dylib_install_name(&alias_dest);
        }
    }
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn fix_dylib_install_name(path: &Path) {
    use std::process::Command;

    let Some(path_str) = path.to_str() else {
        return;
    };

    let status = Command::new("install_name_tool")
        .args(["-id", "@rpath/libndi.dylib", path_str])
        .status();

    if status.is_err() || !status.unwrap().success() {
        println!(
            "cargo:warning=install_name_tool could not set dylib id for {}",
            path.display()
        );
    }
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn resolve_macos_ndi_dylib() -> Option<PathBuf> {
    let sdk_dir = resolve_ndi_sdk_dir()?;
    let lib_macos = sdk_dir.join("lib").join("macOS");
    for candidate in [lib_macos, sdk_dir.join("lib")] {
        if let Some(path) = find_ndi_dylib_in(&candidate) {
            return Some(path);
        }
    }

    None
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn find_ndi_dylib_in(dir: &Path) -> Option<PathBuf> {
    for name in ["libndi.dylib", "libndi.4.dylib"] {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[cfg(all(target_os = "macos", feature = "ndi"))]
fn resolve_ndi_sdk_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("NDI_SDK_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Some(path);
        }
    }

    for candidate in [
        "/Library/NDI SDK for Apple",
        "/Library/NDI 6 SDK",
        "/Library/NDI SDK for macOS",
        "/Library/NDI SDK",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}
