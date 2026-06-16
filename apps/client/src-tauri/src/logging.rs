use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();
static CLIENT_LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(log_dir: &Path) {
    let _ = fs::create_dir_all(log_dir);

    let file_appender = tracing_appender::rolling::daily(log_dir, "tandem.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = LOG_GUARD.set(guard);

    let log_path = log_dir.join("tandem-client.log");
    if let Ok(mut slot) = CLIENT_LOG_PATH.lock() {
        *slot = Some(log_path.clone());
    }

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "=== tandem client log started ===");
    }

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    tracing::info!(log_dir = %log_dir.display(), "file logging enabled");
}

pub fn append_client_log(line: &str) -> Result<(), String> {
    let path = CLIENT_LOG_PATH
        .lock()
        .map_err(|_| "log path lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "client log path not initialized".to_string())?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;

    writeln!(file, "{line}").map_err(|error| error.to_string())
}

pub fn log_path_hint() -> Option<String> {
    CLIENT_LOG_PATH
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|path| path.display().to_string()))
}
