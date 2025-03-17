#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFormat {
    Ttf,
    Otf,
    Woff,
    Woff2,
}

impl FontFormat {
    pub fn mime(&self) -> &'static str {
        match self {
            Self::Ttf => "font/ttf",
            Self::Otf => "font/otf",
            Self::Woff => "font/woff",
            Self::Woff2 => "font/woff2",
        }
    }

    pub fn css(&self) -> &'static str {
        match self {
            Self::Ttf => "truetype",
            Self::Otf => "opentype",
            Self::Woff => "woff",
            Self::Woff2 => "woff2",
        }
    }
}
