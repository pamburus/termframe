use std::path::Path;

use nu_ansi_term::Color;

pub mod suggest;

pub use suggest::Suggestions;

pub trait Highlight {
    fn hl(&self) -> String;
}

impl<S: AsRef<str>> Highlight for S {
    fn hl(&self) -> String {
        HILITE.paint(format!("{:?}", self.as_ref())).to_string()
    }
}

impl Highlight for Path {
    fn hl(&self) -> String {
        self.to_string_lossy().hl()
    }
}

pub const HILITE: Color = Color::Yellow;
