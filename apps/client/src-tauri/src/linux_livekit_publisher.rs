use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitSlotConfig {
    pub slot: String,
    pub ws_url: String,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SidecarMessage<'a> {
    #[serde(rename = "start")]
    Start { url: &'a str, token: &'a str },
    #[serde(rename = "sync")]
    Sync { slots: &'a [LiveKitSlotConfig] },
}

pub struct LinuxLiveKitPublisher {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

impl LinuxLiveKitPublisher {
    pub fn new() -> Self {
        Self {
            child: None,
            stdin: None,
        }
    }

    pub fn start(&mut self, url: &str, token: &str) -> Result<(), String> {
        self.stop();

        let node = find_node_executable()?;
        let script = resolve_script_path()?;
        let client_dir = script
            .parent()
            .and_then(|scripts| scripts.parent())
            .ok_or_else(|| "Could not resolve client directory for LiveKit sidecar".to_string())?;

        tracing::info!(
            node = %node.display(),
            script = %script.display(),
            "starting linux livekit publisher sidecar"
        );

        let mut child = Command::new(&node)
            .arg(&script)
            .current_dir(client_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("Failed to start LiveKit sidecar: {error}"))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "LiveKit sidecar stdin unavailable".to_string())?;

        write_message(
            &mut stdin,
            &SidecarMessage::Start { url, token },
        )?;

        self.stdin = Some(stdin);
        self.child = Some(child);
        Ok(())
    }

    pub fn sync_slots(&mut self, slots: &[LiveKitSlotConfig]) -> Result<(), String> {
        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| "LiveKit sidecar is not running".to_string())?;

        write_message(stdin, &SidecarMessage::Sync { slots })
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.stdin = None;
    }
}

impl Default for LinuxLiveKitPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LinuxLiveKitPublisher {
    fn drop(&mut self) {
        self.stop();
    }
}

fn write_message(stdin: &mut ChildStdin, message: &impl Serialize) -> Result<(), String> {
    let line = serde_json::to_string(message).map_err(|error| error.to_string())?;
    stdin
        .write_all(line.as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("Failed to write LiveKit sidecar message: {error}"))
}

fn find_node_executable() -> Result<PathBuf, String> {
    if let Ok(output) = Command::new("which").arg("node").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    for candidate in ["/usr/bin/node", "/usr/local/bin/node"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err("Node.js executable not found (install node 18+)".to_string())
}

fn resolve_script_path() -> Result<PathBuf, String> {
    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/linux-livekit-publisher.mjs");

    if dev_path.is_file() {
        return dev_path
            .canonicalize()
            .map_err(|error| format!("Failed to resolve LiveKit sidecar script: {error}"));
    }

    Err(format!(
        "LiveKit sidecar script not found at {}",
        dev_path.display()
    ))
}
