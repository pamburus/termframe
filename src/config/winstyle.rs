// std imports
use std::sync::{Arc, LazyLock};

// third-party imports
use csscolorparser::Color;
use rust_embed::RustEmbed;
use serde::Deserialize;

// local imports
use super::{load::Load, mode::Mode};

// ---

// re-exports
pub use super::PaddingOption;

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

    fn dir_name() -> &'static str {
        "window-styles"
    }
}

impl Default for WindowStyleConfig {
    fn default() -> Self {
        DEFAULT.as_ref().clone()
    }
}

impl Default for &WindowStyleConfig {
    fn default() -> Self {
        &*DEFAULT
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Window {
    pub margin: PaddingOption,
    pub border: WindowBorder,
    pub header: WindowHeader,
    pub buttons: WindowButtons,
    pub shadow: WindowShadow,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowBorder {
    pub colors: WindowBorderColors,
    pub width: f32,
    pub radius: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowBorderColors {
    pub outer: SelectiveColor,
    pub inner: SelectiveColor,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowHeader {
    pub color: SelectiveColor,
    pub height: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowButtons {
    pub radius: f32,
    pub spacing: f32,
    pub close: WindowButton,
    pub minimize: WindowButton,
    pub maximize: WindowButton,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowButton {
    pub color: SelectiveColor,
}

#[derive(Debug, Deserialize, Clone)]
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
    LazyLock::new(|| Arc::new(WindowStyleConfig::load("macos").unwrap().ok().unwrap()));
