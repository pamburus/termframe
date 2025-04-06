use std::{
    borrow::Cow,
    fmt::{self, Write as _},
    path::Path,
};

use owo_colors::{OwoColorize, Style};

pub mod suggest;

pub use suggest::Suggestions;

/// A trait for highlighting text.
pub trait Highlight {
    type Output: fmt::Display;

    /// Highlights the text.
    fn hl(self) -> Self::Output;
}

impl<'a, S> Highlight for &'a S
where
    S: fmt::Display,
{
    type Output = Highlighted<&'a S>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

impl<'a> Highlight for &'a Path {
    type Output = Highlighted<Converted<&'a Path>>;

    fn hl(self) -> Self::Output {
        Converted(self).hl()
    }
}

/// A trait for highlighting and quoting text.
pub trait HighlightQuoted {
    type Output: fmt::Display;

    /// Highlights and quotes the text.
    fn hlq(self) -> Self::Output;
}

impl<'a, S> HighlightQuoted for &'a S
where
    S: fmt::Display,
{
    type Output = Highlighted<Quoted<&'a S>>;

    fn hlq(self) -> Self::Output {
        Quoted(self).hl()
    }
}

impl<'a> HighlightQuoted for &'a Path {
    type Output = Highlighted<Quoted<Converted<&'a Path>>>;

    fn hlq(self) -> Self::Output {
        Quoted(Converted(self)).hl()
    }
}

/// A wrapper struct for highlighted text.
pub struct Highlighted<S>(S);

impl<S> fmt::Display for Highlighted<S>
where
    S: fmt::Display + Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.style(HIGHLIGHT))
    }
}

/// A wrapper struct for quoted text.
pub struct Quoted<S>(S);

impl<S> fmt::Display for Quoted<S>
where
    S: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = String::new();
        write!(&mut buf, "{}", self.0)?;
        write!(f, "{:?}", buf)
    }
}

impl<S> Highlight for Quoted<S>
where
    S: fmt::Display,
{
    type Output = Highlighted<Quoted<S>>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

/// A wrapper struct for converted text.
pub struct Converted<T>(T);

impl<T> fmt::Display for Converted<T>
where
    T: HighlightConvert + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.clone().convert())
    }
}

impl<S> Highlight for Converted<S>
where
    S: HighlightConvert + Clone,
{
    type Output = Highlighted<Converted<S>>;

    fn hl(self) -> Self::Output {
        Highlighted(self)
    }
}

/// A trait for converting text for highlighting.
trait HighlightConvert {
    type Output: fmt::Display;

    /// Converts the text for highlighting.
    fn convert(self) -> Self::Output;
}

impl<'a> HighlightConvert for &'a Path {
    type Output = Cow<'a, str>;

    fn convert(self) -> Self::Output {
        self.to_string_lossy()
    }
}

const HIGHLIGHT: Style = Style::new().yellow();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight() {
        assert_eq!("hello".hl().to_string(), "\u{1b}[33mhello\u{1b}[0m");
        assert_eq!(
            Path::new("hello").hl().to_string(),
            "\u{1b}[33mhello\u{1b}[0m"
        );
        assert_eq!("hello".hlq().to_string(), "\u{1b}[33m\"hello\"\u{1b}[0m");
        assert_eq!(
            Path::new("hello").hlq().to_string(),
            "\u{1b}[33m\"hello\"\u{1b}[0m"
        );
    }
}
