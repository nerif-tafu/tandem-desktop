use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    for icon in ["icon.ico", "icon.png", "32x32.png", "128x128.png", "128x128@2x.png"] {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join("icons").join(icon).display()
        );
    }

    copy_ndi_runtime_dll();
    tauri_build::build();
}

fn copy_ndi_runtime_dll() {
    if !cfg!(windows) {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
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
