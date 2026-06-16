#[cfg(feature = "ndi")]
mod imp {
    use std::time::Duration;

    use base64::Engine;
    use grafton_ndi::{
        Finder, FinderOptions, NDI, Receiver, ReceiverBandwidth, ReceiverColorFormat, ReceiverOptions,
    };

    use super::super::sources::CaptureError;
    use super::super::types::{CaptureSource, CaptureSourceKind};

    const DISCOVERY_WAIT: Duration = Duration::from_secs(2);

    pub fn is_available() -> bool {
        NDI::new().is_ok()
    }

    pub fn list_ndi_sources() -> Result<Vec<CaptureSource>, CaptureError> {
        let ndi = NDI::new().map_err(|error| CaptureError::ListFailed(error.to_string()))?;
        let finder = Finder::new(
            &ndi,
            &FinderOptions::builder().show_local_sources(true).build(),
        )
        .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

        let _ = finder
            .wait_for_sources(DISCOVERY_WAIT)
            .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

        let discovered = finder
            .current_sources()
            .map_err(|error| CaptureError::ListFailed(error.to_string()))?;

        Ok(discovered
            .into_iter()
            .map(|source| {
                let name = source.name.clone();
                CaptureSource {
                    id: ndi_source_id(&name),
                    kind: CaptureSourceKind::Ndi,
                    label: name,
                }
            })
            .collect())
    }

    pub fn capture_ndi_preview(source_id: &str) -> Result<String, CaptureError> {
        let mut stream = NdiPreviewStream::open(source_id)?;
        stream.next_frame()
    }

    pub struct NdiPreviewStream {
        _ndi: NDI,
        receiver: Receiver,
    }

    impl NdiPreviewStream {
        pub fn open(source_id: &str) -> Result<Self, CaptureError> {
            let ndi_name = decode_ndi_source_id(source_id)?;
            let ndi = NDI::new().map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;
            let source = find_ndi_source(&ndi, &ndi_name)?;

            let receiver = Receiver::new(
                &ndi,
                &ReceiverOptions::builder(source)
                    .color(ReceiverColorFormat::RGBX_RGBA)
                    .bandwidth(ReceiverBandwidth::Highest)
                    .build(),
            )
            .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

            Ok(Self { _ndi: ndi, receiver })
        }

        pub fn capture_rgba(&mut self) -> Result<image::RgbaImage, CaptureError> {
            let video = self
                .receiver
                .video()
                .capture(Duration::from_millis(34))
                .map_err(|error| CaptureError::CaptureFailed(error.to_string()))?;

            let width = video.width() as u32;
            let height = video.height() as u32;
            image::RgbaImage::from_raw(width, height, video.data().to_vec()).ok_or_else(|| {
                CaptureError::CaptureFailed("invalid NDI frame dimensions".into())
            })
        }

        pub fn next_frame(&mut self) -> Result<String, CaptureError> {
            super::super::sources::encode_preview(self.capture_rgba()?)
        }
    }

    fn find_ndi_source(
        ndi: &NDI,
        ndi_name: &str,
    ) -> Result<grafton_ndi::Source, CaptureError> {
        let finder = Finder::new(
            ndi,
            &FinderOptions::builder().show_local_sources(true).build(),
        )
        .map_err(|error| CaptureError::SourceNotFound(error.to_string()))?;

        let _ = finder
            .wait_for_sources(DISCOVERY_WAIT)
            .map_err(|error| CaptureError::SourceNotFound(error.to_string()))?;

        finder
            .current_sources()
            .map_err(|error| CaptureError::SourceNotFound(error.to_string()))?
            .into_iter()
            .find(|source| source.name == ndi_name)
            .ok_or_else(|| CaptureError::SourceNotFound(ndi_name.to_string()))
    }

    fn ndi_source_id(name: &str) -> String {
        format!(
            "ndi:{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(name.as_bytes())
        )
    }

    fn decode_ndi_source_id(source_id: &str) -> Result<String, CaptureError> {
        let encoded = source_id
            .strip_prefix("ndi:")
            .ok_or_else(|| CaptureError::SourceNotFound(source_id.to_string()))?;

        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| CaptureError::SourceNotFound(error.to_string()))?;

        String::from_utf8(bytes).map_err(|error| CaptureError::SourceNotFound(error.to_string()))
    }
}

#[cfg(not(feature = "ndi"))]
mod imp {
    use super::super::sources::CaptureError;
    use super::super::types::CaptureSource;

    pub fn is_available() -> bool {
        false
    }

    pub fn list_ndi_sources() -> Result<Vec<CaptureSource>, CaptureError> {
        Ok(Vec::new())
    }

    pub fn capture_ndi_preview(_source_id: &str) -> Result<String, CaptureError> {
        Err(CaptureError::Unsupported(
            "NDI support was not compiled into this build".into(),
        ))
    }
}

pub use imp::*;
