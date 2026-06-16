use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CaptureSourceKind {
    Screen,
    Webcam,
    Ndi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub id: String,
    pub kind: CaptureSourceKind,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationWindow {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotCaptureState {
    pub slot: String,
    pub label: String,
    pub source: Option<CaptureSource>,
    pub preview: Option<String>,
    pub active: bool,
}

#[derive(Debug, Default)]
pub struct CaptureManager {
    assignments: HashMap<String, Option<String>>,
}

impl CaptureManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_assignment(&mut self, slot: &str, source_id: Option<String>) {
        self.assignments.insert(slot.to_string(), source_id);
    }

    pub fn get_assignment(&self, slot: &str) -> Option<&Option<String>> {
        self.assignments.get(slot)
    }
}

pub const STREAM_SLOTS: [&str; 4] = ["main", "notes", "aux1", "aux2"];

pub fn slot_label(slot: &str) -> &'static str {
    match slot {
        "main" => "Main presentation",
        "notes" => "Presenter notes",
        "aux1" => "Auxiliary 1",
        "aux2" => "Auxiliary 2",
        _ => "Capture",
    }
}
