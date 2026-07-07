#[cfg(windows)]
mod windows_focus;
#[cfg(windows)]
mod windows_key;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum PresentationError {
    Input(String),
}

impl std::fmt::Display for PresentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresentationError::Input(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PresentationError {}

pub struct KeyboardPresentationController {
    target_window_id: Mutex<Option<u32>>,
    /// Enigo holds non-Send CoreGraphics state on macOS; keep it Windows-only in managed state.
    #[cfg(windows)]
    enigo: Mutex<Enigo>,
    /// Cached input session. Creating it triggers the Wayland "Allow remote
    /// interaction" portal prompt, so it is initialized when a target window
    /// is selected rather than on the first key press.
    #[cfg(target_os = "linux")]
    enigo: Arc<Mutex<Option<Enigo>>>,
}

impl KeyboardPresentationController {
    pub fn new() -> Self {
        Self {
            target_window_id: Mutex::new(None),
            #[cfg(windows)]
            enigo: Mutex::new(
                Enigo::new(&Settings::default()).expect("keyboard controller should initialize"),
            ),
            #[cfg(target_os = "linux")]
            enigo: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_target(&self, source_id: Option<&str>) -> Result<(), PresentationError> {
        let window_id = match source_id {
            None | Some("") => None,
            Some(id) => Some(parse_window_id(id)?),
        };

        *self
            .target_window_id
            .lock()
            .map_err(|_| PresentationError::Input("Keyboard controller is unavailable".into()))? =
            window_id;

        #[cfg(target_os = "linux")]
        if window_id.is_some() {
            self.warm_linux_input_session();
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn warm_linux_input_session(&self) {
        let enigo = self.enigo.clone();
        thread::spawn(move || {
            let Ok(mut guard) = enigo.lock() else {
                return;
            };

            if guard.is_some() {
                return;
            }

            match Enigo::new(&Settings::default()) {
                Ok(instance) => {
                    tracing::info!("presentation input session ready");
                    *guard = Some(instance);
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to initialize presentation input session");
                }
            }
        });
    }

    pub fn get_target(&self) -> Result<Option<String>, PresentationError> {
        let guard = self
            .target_window_id
            .lock()
            .map_err(|_| PresentationError::Input("Keyboard controller is unavailable".into()))?;

        Ok(guard.map(|id| format!("window:{id}")))
    }

    pub fn forward(&self) -> Result<(), PresentationError> {
        self.send_key(Key::RightArrow)
    }

    pub fn back(&self) -> Result<(), PresentationError> {
        self.send_key(Key::LeftArrow)
    }

    fn send_key(&self, key: Key) -> Result<(), PresentationError> {
        let window_id = *self
            .target_window_id
            .lock()
            .map_err(|_| PresentationError::Input("Keyboard controller is unavailable".into()))?;

        #[cfg(windows)]
        if let Some(window_id) = window_id {
            windows_focus::focus_window(window_id)?;
            thread::sleep(Duration::from_millis(75));
            return windows_key::post_presentation_key(window_id, key);
        }

        #[cfg(target_os = "linux")]
        if let Some(window_id) = window_id {
            if crate::linux_gnome_extension::dbus_ping() {
                crate::linux_gnome_extension::activate_window(window_id)
                    .map_err(PresentationError::Input)?;
                thread::sleep(Duration::from_millis(75));
            }
        }

        #[cfg(windows)]
        {
            let mut enigo = self
                .enigo
                .lock()
                .map_err(|_| PresentationError::Input("Keyboard controller is unavailable".into()))?;
            send_enigo_key(&mut enigo, key)?;
        }

        #[cfg(target_os = "linux")]
        {
            let mut guard = self
                .enigo
                .lock()
                .map_err(|_| PresentationError::Input("Keyboard controller is unavailable".into()))?;

            if guard.is_none() {
                *guard = Some(
                    Enigo::new(&Settings::default())
                        .map_err(|error| PresentationError::Input(error.to_string()))?,
                );
            }

            let enigo = guard.as_mut().expect("input session initialized above");
            if let Err(error) = send_cached_key(enigo, key) {
                tracing::warn!(%error, "presentation key failed; rebuilding input session");
                *guard = Some(
                    Enigo::new(&Settings::default())
                        .map_err(|error| PresentationError::Input(error.to_string()))?,
                );
                send_cached_key(guard.as_mut().expect("just replaced"), key)?;
            }
        }

        #[cfg(not(any(windows, target_os = "linux")))]
        send_global_key(key)?;

        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn send_cached_key(enigo: &mut Enigo, key: Key) -> Result<(), PresentationError> {
    enigo
        .key(key, Direction::Press)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    enigo
        .key(key, Direction::Release)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    tracing::info!(?key, "sent presentation key");
    Ok(())
}

#[cfg(windows)]
fn send_enigo_key(enigo: &mut Enigo, key: Key) -> Result<(), PresentationError> {
    enigo
        .key(key, Direction::Press)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    enigo
        .key(key, Direction::Release)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    tracing::info!(?key, "sent presentation key");
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn send_global_key(key: Key) -> Result<(), PresentationError> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    enigo
        .key(key, Direction::Press)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    enigo
        .key(key, Direction::Release)
        .map_err(|error| PresentationError::Input(error.to_string()))?;
    tracing::info!(?key, "sent presentation key");
    Ok(())
}

fn parse_window_id(source_id: &str) -> Result<u32, PresentationError> {
    let raw = source_id
        .strip_prefix("window:")
        .ok_or_else(|| PresentationError::Input("Expected a window source id".into()))?;

    raw.parse()
        .map_err(|_| PresentationError::Input(format!("Invalid window id: {source_id}")))
}
