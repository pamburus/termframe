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

pub struct Options {
    pub font: FontOptions,
    pub line_height: f32,
    pub padding: Padding,
    pub theme: Rc<Theme>,
    pub stroke: f32,
}

pub struct FontOptions {
    pub family: String,
    pub size: f32,
    pub metrics: FontMetrics,
    pub faces: Vec<FontFace>,
}

pub struct FontMetrics {
    pub width: f32,
    pub ascender: f32,
    pub descender: f32,
}

pub struct FontFace {
    pub weight: FontWeight,
    pub style: FontStyle,
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
    Custom(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub x: f32,
    pub y: f32,
}
