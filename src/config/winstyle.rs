// std imports
use std::{
    io,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

// third-party imports
use csscolorparser::Color;
use rust_embed::RustEmbed;
use serde::Deserialize;
use thiserror::Error;

// local imports
use super::{
    load::{self, Categorize, ErrorCategory, Load},
    mode::Mode,
};
use crate::xerr::{Highlight, Suggestions};

// ---

// re-exports
pub use super::PaddingOption;

// ---

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown window style {}", .name.hl())]
    WindowStyleNotFound {
        name: String,
        suggestions: Suggestions,
    },
    #[error("window style file {} not found", .path.hl())]
    WindowStyleFileNotFound { path: PathBuf },
    #[error("invalid window style file path {}", .path.hl())]
    InvalidWindowStyleFilePath { path: PathBuf },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Parse(#[from] load::ParseError),
}

impl From<load::Error> for Error {
    fn from(err: load::Error) -> Self {
        match err {
            load::Error::ItemNotFound {
                name, suggestions, ..
            } => Error::WindowStyleNotFound { name, suggestions },
            load::Error::FileNotFound { path } => Error::WindowStyleFileNotFound { path },
            load::Error::InvalidFilePath { path } => Error::InvalidWindowStyleFilePath { path },
            load::Error::Io(err) => Error::Io(err),
            load::Error::Parse(err) => Error::Parse(err),
        }
    }
}

impl Categorize for Error {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::WindowStyleNotFound { .. } => ErrorCategory::ItemNotFound,
            _ => ErrorCategory::Other,
        }
    }
}

// ---

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowStyleConfig {
    pub window: Window,
}

impl WindowStyleConfig {
    pub fn default(&self) -> Arc<Self> {
        DEFAULT.clone()
    }
}

impl Load for WindowStyleConfig {
    type Assets = Assets;
    type Error = Error;

    fn category() -> &'static str {
        "window styles"
    }

    fn dir_name() -> &'static str {
        "window-styles"
    }

    fn is_not_found_error(err: &Error) -> bool {
        matches!(err, Error::WindowStyleNotFound { .. })
    }
}

impl Default for WindowStyleConfig {
    fn default() -> Self {
        DEFAULT.as_ref().clone()
    }
}

impl Default for &WindowStyleConfig {
    fn default() -> Self {
        &DEFAULT
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Window {
    pub margin: PaddingOption,
    pub border: WindowBorder,
    pub header: WindowHeader,
    pub title: WindowTitle,
    pub buttons: WindowButtons,
    pub shadow: WindowShadow,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowBorder {
    pub colors: WindowBorderColors,
    pub width: f32,
    pub radius: f32,
    pub gap: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowBorderColors {
    pub outer: SelectiveColor,
    pub inner: SelectiveColor,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowHeader {
    pub color: SelectiveColor,
    pub height: f32,
    pub border: Option<WindowHeaderBorder>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowHeaderBorder {
    pub color: SelectiveColor,
    pub width: f32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowTitle {
    pub color: SelectiveColor,
    pub font: Font,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Font {
    pub family: Vec<String>,
    pub size: f32,
    pub weight: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButtons {
    pub position: WindowButtonsPosition,
    pub shape: Option<WindowButtonShape>,
    pub size: f32,
    pub roundness: Option<f32>,
    pub items: Vec<WindowButton>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButton {
    pub offset: f32,
    pub fill: Option<SelectiveColor>,
    pub stroke: Option<SelectiveColor>,
    pub stroke_width: Option<f32>,
    pub icon: Option<WindowButtonIcon>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonsPosition {
    Left,
    Right,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonShape {
    Circle,
    Square,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButtonIcon {
    pub kind: WindowButtonIconKind,
    pub size: f32,
    pub stroke: SelectiveColor,
    pub stroke_width: Option<f32>,
    pub stroke_linecap: Option<LineCap>,
    pub roundness: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonIconKind {
    Close,
    Minimize,
    Maximize,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum LineCap {
    Round,
    Square,
    Butt,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowShadow {
    pub enabled: bool,
    pub color: SelectiveColor,
    pub x: f32,
    pub y: f32,
    pub blur: f32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum SelectiveColor {
    Uniform(Color),
    Adaptive { light: Color, dark: Color },
}

impl SelectiveColor {
    pub fn resolve(&self, mode: Mode) -> &Color {
        match self {
            Self::Uniform(color) => color,
            Self::Adaptive { light, dark } => match mode {
                Mode::Light => light,
                Mode::Dark => dark,
            },
        }
    }
}

// ---

#[derive(RustEmbed)]
#[folder = "src/assets/window-styles/"]
pub struct Assets;

static DEFAULT: LazyLock<Arc<WindowStyleConfig>> =
    LazyLock::new(|| Arc::new(WindowStyleConfig::load("macos").unwrap()));
