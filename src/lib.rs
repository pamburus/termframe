// std imports

// third-party imports
use csscolorparser::Color;
use termwiz::color::SrgbaTuple;

// Public exports
pub mod appdirs;
pub mod cli;
pub mod command;
pub mod config;
pub mod error;
pub mod font;
pub mod fontformat;
pub mod help;
pub mod render;
pub mod syntax;
pub mod term;
pub mod theme;
pub mod ureqmw;
pub mod xerr;

// Re-export key types needed for tests
pub use config::Source;

/// Trait for converting between types
pub trait Convert<T> {
    fn convert(&self) -> T;
}

impl Convert<render::FontWeight> for config::FontWeight {
    fn convert(&self) -> render::FontWeight {
        match self {
            config::FontWeight::Normal => render::FontWeight::Normal,
            config::FontWeight::Bold => render::FontWeight::Bold,
            config::FontWeight::Fixed(weight) => render::FontWeight::Fixed(*weight),
        }
    }
}

impl Convert<render::FontWeights> for config::FontWeights {
    fn convert(&self) -> render::FontWeights {
        render::FontWeights {
            normal: self.normal.convert(),
            bold: self.bold.convert(),
            faint: self.faint.convert(),
        }
    }
}

impl Convert<SrgbaTuple> for Color {
    fn convert(&self) -> SrgbaTuple {
        let x = self.to_rgba8();
        (x[0], x[1], x[2], x[3]).into()
    }
}

impl Convert<Color> for SrgbaTuple {
    fn convert(&self) -> Color {
        self.as_rgba_u8().into()
    }
}
