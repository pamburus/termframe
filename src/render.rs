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
pub mod svg;
mod tracing;

// re-exports
pub type Result<T> = anyhow::Result<T>;

pub trait Render {
    #[allow(dead_code)]
    fn render(&self, surface: &Surface, target: &mut dyn io::Write) -> Result<()>;
}

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
    pub fn bg(&self) -> &Color {
        self.background.as_ref().unwrap_or(&self.theme.bg)
    }

    pub fn fg(&self) -> &Color {
        self.foreground.as_ref().unwrap_or(&self.theme.fg)
    }
}

#[derive(Debug, Clone)]
pub struct FontOptions {
    pub family: Vec<String>,
    pub size: f32,
    pub metrics: FontMetrics,
    pub faces: Vec<FontFace>,
    pub weights: FontWeights,
}

#[derive(Debug, Clone)]
pub struct FontMetrics {
    pub width: f32,
    pub ascender: f32,
    pub descender: f32,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum FontWeight {
    Normal,
    Bold,
    Fixed(u16),
    Variable(u16, u16),
}

impl FontWeight {
    pub fn range(&self) -> (u16, u16) {
        match self {
            Self::Normal => (400, 400),
            Self::Bold => (600, 600),
            Self::Fixed(weight) => (*weight, *weight),
            Self::Variable(min, max) => (*min, *max),
        }
    }
}

// ---

pub trait CharSet: std::fmt::Debug {
    fn has_char(&self, ch: char) -> bool;
}

impl CharSet for HashSet<char> {
    fn has_char(&self, ch: char) -> bool {
        self.contains(&ch)
    }
}

// ---

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
    pub fn new(f: F) -> Self {
        Self(f)
    }
}
