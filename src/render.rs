// std imports
use std::{collections::HashSet, rc::Rc};

// third-party imports
use termwiz::surface::Surface;

// local imports
use crate::{
    Theme,
    config::{Padding, Settings, mode::Mode, winstyle::Window},
};

pub mod svg;

// re-exports
pub type Result<T> = anyhow::Result<T>;

pub trait Render {
    #[allow(dead_code)]
    fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Options {
    pub settings: Rc<Settings>,
    pub font: FontOptions,
    pub theme: Rc<Theme>,
    pub window: Window,
    pub mode: Mode,
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
    pub format: Option<&'static str>,
    pub chars: Rc<dyn CharSet>,
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
