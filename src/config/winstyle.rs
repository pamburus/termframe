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
    /// Error when the window style is not found.
    #[error("unknown window style {}", .name.hlq())]
    WindowStyleNotFound {
        name: Arc<str>,
        suggestions: Suggestions,
    },

    /// Error when the window style file is not found.
    #[error("window style file {} not found", .path.hlq())]
    WindowStyleFileNotFound { path: PathBuf },

    /// Error when the window style file path is invalid.
    #[error("invalid window style file path {}", .path.hlq())]
    InvalidWindowStyleFilePath { path: PathBuf },

    /// Error when failing to list window styles.
    #[error("failed to list window styles: {source}")]
    FailedToListWindowStyles { source: io::Error },

    /// Error when failing to load a window style.
    #[error("failed to load window style {name}: {source}", name=.name.hlq())]
    Io { name: Arc<str>, source: io::Error },

    /// Error when failing to parse a window style.
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

/// Configuration for window styles.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowStyleConfig {
    pub window: Window,
}

impl WindowStyleConfig {
    /// Returns the default window style configuration.
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

/// Configuration for a window.
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

/// Configuration for a window border.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowBorder {
    pub colors: WindowBorderColors,
    pub width: Number,
    pub radius: Number,
    pub gap: Option<Number>,
}

/// Colors for a window border.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowBorderColors {
    pub outer: SelectiveColor,
    pub inner: SelectiveColor,
}

/// Configuration for a window header.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowHeader {
    pub color: SelectiveColor,
    pub height: Number,
    pub border: Option<WindowHeaderBorder>,
}

/// Configuration for a window header border.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowHeaderBorder {
    pub color: SelectiveColor,
    pub width: Number,
}

/// Configuration for a window title.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowTitle {
    pub color: SelectiveColor,
    pub font: Font,
}

/// Configuration for a font.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Font {
    pub family: Vec<String>,
    pub size: Number,
    pub weight: Option<String>,
}

/// Configuration for window buttons.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButtons {
    pub position: WindowButtonsPosition,
    pub shape: Option<WindowButtonShape>,
    pub size: Number,
    pub roundness: Option<Number>,
    pub items: Vec<WindowButton>,
}

/// Configuration for a window button.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowButton {
    pub offset: Number,
    pub fill: Option<SelectiveColor>,
    pub stroke: Option<SelectiveColor>,
    pub stroke_width: Option<Number>,
    pub icon: Option<WindowButtonIcon>,
}

/// Position of window buttons.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonsPosition {
    Left,
    Right,
}

/// Shape of a window button.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonShape {
    Circle,
    Square,
}

/// Icon for a window button.
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

/// Kind of window button icon.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WindowButtonIconKind {
    Close,
    Minimize,
    Maximize,
}

/// Line cap style for a window button icon.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum LineCap {
    Round,
    Square,
    Butt,
}

/// Configuration for a window shadow.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WindowShadow {
    pub enabled: bool,
    pub color: SelectiveColor,
    pub x: Number,
    pub y: Number,
    pub blur: Number,
}

/// Color that can be either uniform or adaptive based on the mode.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum SelectiveColor {
    Uniform(Color),
    Adaptive { light: Color, dark: Color },
}

impl SelectiveColor {
    /// Resolves the color based on the mode.
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

/// Embeds the assets for window styles.
#[derive(RustEmbed)]
#[folder = "assets/window-styles/"]
pub struct Assets;

static DEFAULT: LazyLock<Arc<WindowStyleConfig>> =
    LazyLock::new(|| Arc::new(WindowStyleConfig::load("macos").unwrap()));
