// std imports
use std::{collections::HashSet, io, rc::Rc};

// third-party imports
use csscolorparser::Color;
use termwiz::surface::Surface;

// local imports
use crate::{
    Theme,
    config::{Padding, Settings, mode::Mode, winstyle::Window},
    fontformat::FontFormat,
};

// modules
pub mod png;
pub mod svg;
mod tracing;

// re-exports
pub type Result<T> = anyhow::Result<T>;

/// Trait for rendering objects onto a surface.
pub trait Render {
    /// Render the object onto the given surface and write the output to the target.
    #[allow(dead_code)]
    fn render(&self, surface: &Surface, target: &mut dyn io::Write) -> Result<()>;
}

/// Options for configuring the rendering `environment.
#[derive(Debug, Clone)]
pub struct Options {
    pub settings: Rc<Settings>,
    pub font: FontOptions,
    pub theme: Rc<Theme>,
    pub window: Window,
    pub title: Option<String>,
    pub mode: Mode,
    pub background: Option<Color>,
    pub foreground: Option<Color>,
}

impl Options {
    /// Get the background color, falling back to the theme's background color if not set.
    pub fn bg(&self) -> &Color {
        self.background.as_ref().unwrap_or(&self.theme.bg)
    }

    /// Get the foreground color, falling back to the theme's foreground color if not set.
    pub fn fg(&self) -> &Color {
        self.foreground.as_ref().unwrap_or(&self.theme.fg)
    }
}

/// Options for configuring font properties.
#[derive(Debug, Clone)]
pub struct FontOptions {
    pub family: Vec<String>,
    pub size: f32,
    pub metrics: FontMetrics,
    pub faces: Vec<FontFace>,
    pub weights: FontWeights,
}

/// Metrics for font dimensions.
#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub width: f32,
    pub ascender: f32,
    pub descender: f32,
}

/// Weights for different font styles.
#[derive(Debug, Clone)]
pub struct FontWeights {
    pub normal: FontWeight,
    pub bold: FontWeight,
    pub faint: FontWeight,
}

impl Default for FontWeights {
    fn default() -> Self {
        Self {
            normal: FontWeight::Normal,
            bold: FontWeight::Bold,
            faint: FontWeight::Normal,
        }
    }
}

/// Representation of a font face.
#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub weight: FontWeight,
    pub style: Option<FontStyle>,
    pub url: String,
    pub format: Option<FontFormat>,
    pub chars: Rc<dyn CharSet>,
    pub metrics_match: bool,
}

/// Enum representing different font styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Enum representing different font weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum FontWeight {
    Normal,
    Bold,
    Fixed(u16),
    Variable(u16, u16),
}

impl FontWeight {
    /// Get the range of the font weight.
    pub fn range(&self) -> (u16, u16) {
        match self {
            Self::Normal => (400, 400),
            Self::Bold => (600, 600),
            Self::Fixed(weight) => (*weight, *weight),
            Self::Variable(min, max) => (*min, *max),
        }
    }
}

/// Trait for character sets.
pub trait CharSet: std::fmt::Debug {
    /// Check if the character set contains the given character.
    fn has_char(&self, ch: char) -> bool;
}

impl CharSet for HashSet<char> {
    fn has_char(&self, ch: char) -> bool {
        self.contains(&ch)
    }
}

/// Wrapper for a function that checks if a character is in a set.
pub struct CharSetFn<F>(F);

impl<F> std::fmt::Debug for CharSetFn<F>
where
    F: Fn(char) -> bool,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharSetFn").finish()
    }
}

impl<F> CharSet for CharSetFn<F>
where
    F: Fn(char) -> bool,
{
    fn has_char(&self, ch: char) -> bool {
        self.0(ch)
    }
}

impl<F> CharSetFn<F> {
    /// Create a new CharSetFn from a function.
    pub fn new(f: F) -> Self {
        Self(f)
    }
}
