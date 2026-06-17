#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    tandem_client_lib::linux_appimage_env::prepare();

    tandem_client_lib::run();
}
