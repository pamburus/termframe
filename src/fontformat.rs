/// Enum representing different font formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFormat {
    /// TrueType font format.
    Ttf,
    /// OpenType font format.
    Otf,
    /// Web Open Font Format.
    Woff,
    /// Web Open Font Format 2.
    Woff2,
}

impl FontFormat {
    /// Returns the MIME type associated with the font format.
    ///
    /// # Examples
    ///
    /// ```
    /// use termframe::fontformat::FontFormat;
    /// let format = FontFormat::Ttf;
    /// assert_eq!(format.mime(), "font/ttf");
    /// ```
    pub fn mime(&self) -> &'static str {
        match self {
            Self::Ttf => "font/ttf",
            Self::Otf => "font/otf",
            Self::Woff => "font/woff",
            Self::Woff2 => "font/woff2",
        }
    }

    /// Returns the CSS font format string associated with the font format.
    ///
    /// # Examples
    ///
    /// ```
    /// use termframe::fontformat::FontFormat;
    /// let format = FontFormat::Ttf;
    /// assert_eq!(format.css(), "truetype");
    /// ```
    pub fn css(&self) -> &'static str {
        match self {
            Self::Ttf => "truetype",
            Self::Otf => "opentype",
            Self::Woff => "woff",
            Self::Woff2 => "woff2",
        }
    }
}
