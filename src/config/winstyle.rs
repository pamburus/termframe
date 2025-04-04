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
    types::Number,
};
use crate::xerr::{HighlightQuoted, Suggestions};

// ---

// re-exports
pub use super::PaddingOption;

// ---

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown window style {}", .name.hlq())]
    WindowStyleNotFound {
        name: Arc<str>,
        suggestions: Suggestions,
    },
    #[error("window style file {} not found", .path.hlq())]
    WindowStyleFileNotFound { path: PathBuf },
    #[error("invalid window style file path {}", .path.hlq())]
    InvalidWindowStyleFilePath { path: PathBuf },
    #[error("failed to list window styles: {source}")]
    FailedToListWindowStyles { source: io::Error },
    #[error("failed to load window style {name}: {source}", name=.name.hlq())]
    Io { name: Arc<str>, source: io::Error },
    #[error("failed to parse window style {name}: {source}", name=.name.hlq())]
    FailedToParseWindowStyle {
        name: Arc<str>,
        source: load::ParseError,
    },
}

impl From<load::Error> for Error {
    fn from(err: load::Error) -> Self {
        match err {
            load::Error::ItemNotFound {
                name, suggestions, ..
            } => Self::WindowStyleNotFound { name, suggestions },
            load::Error::FileNotFound { path } => Self::WindowStyleFileNotFound { path },
            load::Error::InvalidFilePath { path } => Self::InvalidWindowStyleFilePath { path },
            load::Error::FailedToListItems { source, .. } => {
                Self::FailedToListWindowStyles { source }
            }
            load::Error::Io { name, source, .. } => Self::Io { name, source },
            load::Error::Parse { name, source, .. } => {
                Self::FailedToParseWindowStyle { name, source }
            }
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
    #[serde(default)]
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
    pub width: Number,
    pub radius: Number,
    pub gap: Option<Number>,
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
    pub height: Number,
    pub border: Option<WindowHeaderBorder>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowHeaderBorder {
    pub color: SelectiveColor,
    pub width: Number,
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
    pub size: Number,
    pub weight: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButtons {
    pub position: WindowButtonsPosition,
    pub shape: Option<WindowButtonShape>,
    pub size: Number,
    pub roundness: Option<Number>,
    pub items: Vec<WindowButton>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButton {
    pub offset: Number,
    pub fill: Option<SelectiveColor>,
    pub stroke: Option<SelectiveColor>,
    pub stroke_width: Option<Number>,
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
    pub size: Number,
    pub stroke: SelectiveColor,
    pub stroke_width: Option<Number>,
    pub stroke_linecap: Option<LineCap>,
    pub roundness: Option<Number>,
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
    pub x: Number,
    pub y: Number,
    pub blur: Number,
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
#[folder = "assets/window-styles/"]
pub struct Assets;

static DEFAULT: LazyLock<Arc<WindowStyleConfig>> =
    LazyLock::new(|| Arc::new(WindowStyleConfig::load("macos").unwrap()));
