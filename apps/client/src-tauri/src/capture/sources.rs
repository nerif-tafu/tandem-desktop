use super::ndi::{capture_ndi_preview, list_ndi_sources};
use super::types::{CaptureSource, CaptureSourceKind};

#[derive(Debug)]
pub enum CaptureError {
    ListFailed(String),
    CaptureFailed(String),
    SourceNotFound(String),
    Unsupported(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::ListFailed(message) => write!(f, "Failed to list sources: {message}"),
            CaptureError::CaptureFailed(message) => write!(f, "Failed to capture preview: {message}"),
            CaptureError::SourceNotFound(message) => write!(f, "Source not found: {message}"),
            CaptureError::Unsupported(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CaptureError {}

pub fn list_all_sources() -> Result<Vec<CaptureSource>, CaptureError> {
    let mut sources = Vec::new();
    sources.extend(list_screens()?);
    sources.extend(list_webcams()?);
    sources.extend(match list_ndi_sources() {
        Ok(ndi_sources) => ndi_sources,
        Err(error) => {
            tracing::warn!(%error, "ndi source discovery failed");
            Vec::new()
        }
    });
    Ok(sources)
}

pub fn list_presentation_windows() -> Result<Vec<super::types::PresentationWindow>, CaptureError> {
    let mut windows = Vec::new();

    for window in xcap::Window::all().map_err(|error| CaptureError::ListFailed(error.to_string()))? {
        let title = window
            .title()
            .map_err(|error| CaptureError::ListFailed(error.to_string()))?;
        if title.trim().is_empty() {
            continue;
        }

        let id = window
            .id()
            .map_err(|error| CaptureError::ListFailed(error.to_string()))?;
        let app_name = window
            .app_name()
            .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

        windows.push(super::types::PresentationWindow {
            id: format!("window:{id}"),
            label: format!("{title} — {app_name}"),
        });
    }

    Ok(windows)
}

pub fn find_source(source_id: &str) -> Result<CaptureSource, CaptureError> {
    list_all_sources()?
        .into_iter()
        .find(|source| source.id == source_id)
        .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))
}

pub fn capture_preview(source_id: &str) -> Result<String, CaptureError> {
    let source = find_source(source_id)?;

    match source.kind {
        CaptureSourceKind::Screen => capture_monitor_preview(source_id),
        CaptureSourceKind::Webcam => capture_webcam_preview(source_id),
        CaptureSourceKind::Ndi => capture_ndi_preview(source_id),
    }
}

fn list_screens() -> Result<Vec<CaptureSource>, CaptureError> {
    #[cfg(windows)]
    {
        use windows_capture::monitor::Monitor;

        return Ok(
            Monitor::enumerate()
                .map_err(|error| CaptureError::ListFailed(error.to_string()))?
                .into_iter()
                .enumerate()
                .map(|(index, monitor)| {
                    let screen_index = index + 1;
                    let name = monitor.name().unwrap_or_else(|_| format!("Monitor {screen_index}"));
                    let width = monitor.width().unwrap_or(0);
                    let height = monitor.height().unwrap_or(0);

                    CaptureSource {
                        id: format!("screen:{screen_index}"),
                        kind: CaptureSourceKind::Screen,
                        label: format!("{name} ({width}×{height})"),
                    }
                })
                .collect(),
        );
    }

    #[cfg(not(windows))]
    {
        let mut sources = Vec::new();

        for monitor in
            xcap::Monitor::all().map_err(|error| CaptureError::ListFailed(error.to_string()))?
        {
            let id = monitor
                .id()
                .map_err(|error| CaptureError::ListFailed(error.to_string()))?;
            let name = monitor
                .name()
                .map_err(|error| CaptureError::ListFailed(error.to_string()))?;
            let width = monitor
                .width()
                .map_err(|error| CaptureError::ListFailed(error.to_string()))?;
            let height = monitor
                .height()
                .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

            sources.push(CaptureSource {
                id: format!("screen:{id}"),
                kind: CaptureSourceKind::Screen,
                label: format!("{name} ({width}×{height})"),
            });
        }

        Ok(sources)
    }
}

fn list_webcams() -> Result<Vec<CaptureSource>, CaptureError> {
    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

    Ok(cameras
        .into_iter()
        .enumerate()
        .map(|(index, camera)| CaptureSource {
            id: format!("webcam:{index}"),
            kind: CaptureSourceKind::Webcam,
            label: camera.human_name(),
        })
        .collect())
}

fn capture_monitor_preview(source_id: &str) -> Result<String, CaptureError> {
    #[cfg(windows)]
    {
        let index = parse_id_suffix(source_id, "screen:")? as usize;
        let monitor = xcap::Monitor::all()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .into_iter()
            .nth(index.saturating_sub(1))
            .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?;

        let image = monitor
            .capture_image()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        return encode_preview(image);
    }

    #[cfg(not(windows))]
    {
        let monitor_id = parse_id_suffix(source_id, "screen:")?;

        let monitor = xcap::Monitor::all()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?
            .into_iter()
            .find(|monitor| {
                monitor
                    .id()
                    .map(|id| id == monitor_id)
                    .unwrap_or(false)
            })
            .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?;

        let image = monitor
            .capture_image()
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

        encode_preview(image)
    }
}

fn capture_webcam_preview(source_id: &str) -> Result<String, CaptureError> {
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{CameraFormat, FrameFormat, RequestedFormat, RequestedFormatType, Resolution};

    let index = parse_id_suffix(source_id, "webcam:")? as usize;

    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let camera_info = cameras
        .get(index)
        .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?;

    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(CameraFormat::new(
        Resolution::new(PREVIEW_MAX_WIDTH, PREVIEW_MAX_HEIGHT),
        FrameFormat::MJPEG,
        30,
    )));

    let mut camera = nokhwa::Camera::new(camera_info.index().clone(), requested)
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    camera
        .open_stream()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let frame = camera
        .frame()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let _ = camera.stop_stream();

    let decoded = frame
        .decode_image::<RgbFormat>()
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    let rgba = image::DynamicImage::ImageRgb8(decoded).to_rgba8();
    encode_preview(rgba)
}

pub(crate) const PREVIEW_MAX_WIDTH: u32 = 1280;
pub(crate) const PREVIEW_MAX_HEIGHT: u32 = 720;
pub(crate) const PREVIEW_JPEG_QUALITY: u8 = 82;

pub(crate) fn parse_id_suffix(source_id: &str, prefix: &str) -> Result<u32, CaptureError> {
    source_id
        .strip_prefix(prefix)
        .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?
        .parse::<u32>()
        .map_err(|error| CaptureError::SourceNotFound(error.to_string()))
}

pub(crate) fn encode_preview_jpeg_bytes(image: image::RgbaImage) -> Result<Vec<u8>, CaptureError> {
    use image::codecs::jpeg::JpegEncoder;

    let scaled = if image.width() <= PREVIEW_MAX_WIDTH && image.height() <= PREVIEW_MAX_HEIGHT {
        image
    } else {
        image::imageops::thumbnail(&image, PREVIEW_MAX_WIDTH, PREVIEW_MAX_HEIGHT)
    };

    let rgb = image::DynamicImage::ImageRgba8(scaled).into_rgb8();
    let mut buffer = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buffer, PREVIEW_JPEG_QUALITY);

    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

    Ok(buffer)
}

pub(crate) fn encode_preview(image: image::RgbaImage) -> Result<String, CaptureError> {
    use base64::Engine;

    let buffer = encode_preview_jpeg_bytes(image)?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buffer)
    ))
}
