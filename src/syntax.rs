//! Syntax highlighting support using lumis/syntect.
//!
//! This module provides a facade for syntax highlighting functionality,
//! encapsulating the lumis crate and handling theme name transformations.

use std::fmt;

pub use lumis::languages::Language;
use lumis::{TerminalBuilder, formatter::Formatter};

/// A syntax highlighting theme.
///
/// Theme names use `-` as separator externally (e.g., "aura-dark"),
/// but are transformed to `_` internally for compatibility with lumis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    inner: lumis::themes::Theme,
    display_name: String,
}

impl Theme {
    /// Parses a theme from a string.
    ///
    /// The theme name is expected to use `-` as separator (e.g., "aura-dark").
    /// It will be transformed to `_` internally for compatibility with lumis.
    pub fn parse(name: &str) -> Result<Self, ThemeParseError> {
        let internal_name = name.replace('-', "_");
        let inner = internal_name
            .parse::<lumis::themes::Theme>()
            .map_err(|e| ThemeParseError {
                name: name.to_string(),
                source: e,
            })?;

        Ok(Self {
            inner,
            display_name: name.to_string(),
        })
    }

    /// Returns the display name of the theme (using `-` separator).
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl std::str::FromStr for Theme {
    type Err = ThemeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Error type for theme parsing failures.
#[derive(Debug)]
pub struct ThemeParseError {
    name: String,
    source: lumis::themes::ThemeParseError,
}

impl fmt::Display for ThemeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse theme '{}': {}", self.name, self.source)
    }
}

impl std::error::Error for ThemeParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Information about an available theme.
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// The theme name (using `-` separator).
    pub name: String,
    /// The theme appearance (dark or light).
    pub appearance: Appearance,
}

/// Theme appearance type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Appearance {
    Dark,
    Light,
}

impl fmt::Display for Appearance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Appearance::Dark => write!(f, "dark"),
            Appearance::Light => write!(f, "light"),
        }
    }
}

impl From<lumis::themes::Appearance> for Appearance {
    fn from(appearance: lumis::themes::Appearance) -> Self {
        match appearance {
            lumis::themes::Appearance::Dark => Appearance::Dark,
            lumis::themes::Appearance::Light => Appearance::Light,
        }
    }
}

/// Returns an iterator over all available themes.
///
/// Theme names are transformed to use `-` separator instead of `_`.
pub fn available_themes() -> impl Iterator<Item = ThemeInfo> {
    lumis::themes::available_themes().map(|t| ThemeInfo {
        name: t.name.replace('_', "-"),
        appearance: t.appearance.into(),
    })
}

/// A syntax highlighter for terminal output.
pub struct Highlighter {
    language: Language,
    theme: Option<lumis::themes::Theme>,
}

impl Highlighter {
    /// Creates a new highlighter with the specified language and optional theme.
    pub fn new(language: Language, theme: Option<Theme>) -> Self {
        Self {
            language,
            theme: theme.map(|t| t.inner),
        }
    }

    /// Formats the input text with syntax highlighting to the output.
    pub fn format(&self, input: &str, output: &mut Vec<u8>) -> Result<(), FormatError> {
        let mut builder = TerminalBuilder::new();
        builder.lang(self.language);
        builder.theme(self.theme.clone());

        builder
            .build()
            .map_err(|e| FormatError(e.to_string()))?
            .format(input, output)
            .map_err(|e| FormatError(e.to_string()))
    }
}

/// Error type for formatting failures.
#[derive(Debug)]
pub struct FormatError(String);

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formatting error: {}", self.0)
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests;
