// std imports
use std::rc::Rc;

// third-party imports
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
}

#[derive(Debug, Clone)]
pub struct FontOptions {
    pub family: String,
    pub size: f32,
    pub metrics: FontMetrics,
    pub faces: Vec<FontFace>,
}

#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    pub width: f32,
    pub ascender: f32,
    pub descender: f32,
}

#[derive(Debug, Clone)]
pub struct FontFace {
    pub weight: FontWeight,
    pub style: Option<FontStyle>,
    pub url: String,
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
            Self::Normal => (400, 500),
            Self::Bold => (600, 700),
            Self::Fixed(weight) => (*weight, *weight),
            Self::Variable(min, max) => (*min, *max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub x: f32,
    pub y: f32,
}
