// std imports
use std::{collections::HashSet, rc::Rc};

// third-party imports
use csscolorparser::Color;
use termwiz::surface::Surface;

// local imports
use crate::Theme;

pub mod svg;

// re-exports
pub type Result<T> = anyhow::Result<T>;

pub trait Render {
    #[allow(dead_code)]
    fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Options {
    pub font: FontOptions,
    pub line_height: f32,
    pub padding: Padding,
    pub theme: Rc<Theme>,
    pub precision: u8,
    pub stroke: f32,
    pub faint_opacity: f32,
    pub bold_is_bright: bool,
    pub window: Window,
}

#[derive(Debug, Clone)]
pub struct FontOptions {
    pub family: String,
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
    pub weight: FontWeight,
    pub style: Option<FontStyle>,
    pub url: String,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

// ---

#[derive(Debug, Clone)]
pub struct Window {
    pub enabled: bool,
    pub margin: Padding,
    pub border: WindowBorder,
    pub header: WindowHeader,
    pub buttons: WindowButtons,
    pub shadow: WindowShadow,
}

#[derive(Debug, Clone)]
pub struct WindowBorder {
    pub color1: Color,
    pub color2: Color,
    pub width: f32,
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct WindowHeader {
    pub color: Color,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct WindowButtons {
    pub radius: f32,
    pub spacing: f32,
    pub close: WindowButton,
    pub minimize: WindowButton,
    pub maximize: WindowButton,
}

#[derive(Debug, Clone)]
pub struct WindowShadow {
    pub enabled: bool,
    pub color: Color,
    pub blur: f32,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct WindowButton {
    pub color: Color,
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
