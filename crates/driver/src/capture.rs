use crate::element::ElementRect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Webp,
}

impl ScreenshotFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenshotFormat::Png => "png",
            ScreenshotFormat::Jpeg => "jpeg",
            ScreenshotFormat::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotOptions {
    pub format: ScreenshotFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<ElementRect>,
    pub from_surface: bool,
    pub capture_beyond_viewport: bool,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            format: ScreenshotFormat::Png,
            quality: None,
            clip: None,
            from_surface: true,
            capture_beyond_viewport: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScreencastFrame {
    pub data: Vec<u8>,
    pub session_id: u32,
    pub timestamp: Option<f64>,
}
