// std imports
use std::{path::PathBuf, sync::LazyLock, time::Duration};

// third-party imports
use allsorts::{
    binary::read::{ReadScope, ReadScopeOwned},
    font::MatchingPresentation,
    font_data::{DynamicFontTableProvider, FontData},
    subset::subset,
    tables::{FontTableProvider, HeadTable, NameTable, os2::Os2},
    tag,
};
use anyhow::anyhow;
use exponential_backoff::Backoff;
use url::Url;

// local imports
use crate::fontformat::FontFormat;

// retry loop backoff configuration
static BACKOFF: LazyLock<Backoff> =
    LazyLock::new(|| Backoff::new(8, Duration::from_secs(1), Some(Duration::from_secs(15))));

/// Represents a font file with its location and data.
#[allow(dead_code)]
pub struct FontFile {
    location: Location,
    data: ReadScopeOwned,
}

pub type Result<T> = anyhow::Result<T>;
pub type Fixed = allsorts::tables::Fixed;

impl FontFile {
    /// Load a font file from a given location.
    #[allow(dead_code)]
    pub fn load(location: Location) -> Result<Self> {
        match location {
            Location::File(path) => Self::load_file(path),
            Location::Url(url) => Self::load_url(url),
        }
    }

    /// Load a font file from a file path.
    pub fn load_file(path: PathBuf) -> Result<Self> {
        let bytes = std::fs::read(&path)?;
        Self::load_bytes(&bytes, Location::File(path))
    }

    /// Load a font file from a URL.
    pub fn load_url(url: Url) -> Result<Self> {
        Self::load_url_with_agent(url, &ureq::Agent::new_with_defaults())
    }

    /// Load a font file from a URL using a specific agent.
    pub fn load_url_with_agent(url: Url, agent: &ureq::Agent) -> Result<Self> {
        match url.scheme() {
            "file" | "" => Self::load_file(url.path().into()),
            _ => {
                let attempt = || agent.get(url.as_ref()).call()?.body_mut().read_to_vec();

                let mut result = Err(ureq::Error::Other(
                    anyhow!("Backoff loop malfunction").into(),
                ));

                for delay in BACKOFF.iter() {
                    result = attempt();
                    match &result {
                        Ok(_) => break,
                        Err(ureq::Error::StatusCode(..=428 | 430..=499 | 501 | 505..)) => break,
                        Err(e) => match delay {
                            Some(delay) => {
                                log::debug!("Retry in {delay:?} due to {e:?}");
                                std::thread::sleep(delay)
                            }
                            None => {
                                log::debug!("Retry limit exceeded");
                                break;
                            }
                        },
                    }
                }

                Self::load_bytes(&result?, Location::Url(url))
            }
        }
    }

    /// Load a font file from raw bytes.
    pub fn load_bytes(bytes: &[u8], location: Location) -> Result<Self> {
        let data = ReadScopeOwned::new(ReadScope::new(bytes));
        Ok(Self { location, data })
    }

    /// Get the raw data of the font file.
    pub fn data(&self) -> &[u8] {
        self.data.scope().data()
    }

    /// Determine the format of the font file.
    pub fn format(&self) -> Option<FontFormat> {
        if self.data().len() < 4 {
            return None;
        }
        match &self.data()[0..4] {
            b"\x00\x01\x00\x00" => Some(FontFormat::Ttf),
            b"OTTO" => Some(FontFormat::Otf),
            b"ttcf" => Some(FontFormat::Ttf),
            b"wOFF" => Some(FontFormat::Woff),
            b"wOF2" => Some(FontFormat::Woff2),
            _ => None,
        }
    }

    /// Get the location of the font file.
    #[allow(dead_code)]
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// Get the font object from the font file.
    pub fn font(&self) -> Result<Font> {
        let provider = self.data.scope().read::<FontData>()?.table_provider(0)?;

        let name_data = provider.read_table_data(tag::NAME)?;
        let name_table = ReadScope::new(name_data.as_ref()).read::<NameTable>()?;
        let name = name_table.string_for_id(1);
        let family = name_table.string_for_id(16);

        let inner = allsorts::Font::new(provider)?;
        let Some(head) = inner.head_table()? else {
            return Err(anyhow!("No head table found in the font"));
        };
        let Some(os2) = inner.os2_table()? else {
            return Err(anyhow!("No os/2 table found in the font"));
        };
        Ok(Font {
            inner,
            head,
            os2,
            format: self.format(),
            name,
            family,
        })
    }
}

/// Represents the location of a font file, either a file path or a URL.
#[derive(Debug, Clone)]
pub enum Location {
    File(PathBuf),
    Url(Url),
}

impl Location {
    /// Automatically determine the location type from a string.
    pub fn auto<S: AsRef<str>>(s: S) -> Self {
        match Url::parse(s.as_ref()) {
            Ok(url) => Self::Url(url),
            Err(_) => Self::File(PathBuf::from(s.as_ref())),
        }
    }

    /// Get the URL if the location is a URL.
    pub fn url(&self) -> Option<&Url> {
        match self {
            Self::Url(url) => Some(url),
            Self::File(_) => None,
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::File(path) => write!(f, "{}", path.display()),
            Self::Url(url) => write!(f, "{}", url),
        }
    }
}

impl From<PathBuf> for Location {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

impl From<Url> for Location {
    fn from(url: Url) -> Self {
        Self::Url(url)
    }
}

impl From<&str> for Location {
    fn from(s: &str) -> Self {
        Self::auto(s)
    }
}

/// Represents a font with its various properties and methods.
#[allow(dead_code)]
pub struct Font<'a> {
    inner: allsorts::Font<DynamicFontTableProvider<'a>>,
    head: HeadTable,
    os2: Os2,
    format: Option<FontFormat>,
    name: Option<String>,
    family: Option<String>,
}

impl Font<'_> {
    /// Get the format of the font.
    pub fn format(&self) -> Option<FontFormat> {
        self.format
    }

    /// Get the family name of the font.
    pub fn family(&self) -> Option<&str> {
        self.family.as_deref()
    }

    /// Get the name of the font.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the width of the '0' glyph in the font.
    pub fn width(&mut self) -> f32 {
        let (glyph, _) = self
            .inner
            .lookup_glyph_index('0', MatchingPresentation::Required, None);
        self.inner
            .horizontal_advance(glyph)
            .map(|x| x as f32 / self.em() as f32)
            .unwrap_or(1.0)
    }

    /// Get the ascender value of the font.
    pub fn ascender(&self) -> f32 {
        self.inner.hhea_table.ascender as f32 / self.em() as f32
    }

    /// Get the descender value of the font.
    pub fn descender(&self) -> f32 {
        self.inner.hhea_table.descender as f32 / self.em() as f32
    }

    /// Get the line gap value of the font.
    #[allow(dead_code)]
    pub fn line_gap(&self) -> f32 {
        self.inner.hhea_table.line_gap as f32 / self.em() as f32
    }

    /// Get the weight class of the font.
    #[allow(dead_code)]
    pub fn weight(&self) -> u16 {
        self.os2.us_weight_class
    }

    /// Check if the font is italic.
    pub fn italic(&self) -> bool {
        self.head.is_italic()
    }

    /// Check if the font is bold.
    pub fn bold(&self) -> bool {
        self.head.is_bold()
    }

    /// Get the weight axis range of the font.
    pub fn weight_axis(&self) -> Option<(Fixed, Fixed)> {
        self.axis(tag::WGHT)
    }

    /// Check if the font has an italic axis.
    pub fn has_italic_axis(&self) -> bool {
        self.axis(tag::ITAL).is_some()
    }

    /// Check if the font contains a specific character.
    pub fn has_char(&mut self, ch: char) -> bool {
        self.glyph_index(ch).is_some()
    }

    /// Create a subset of the font containing only the specified characters.
    #[allow(dead_code)]
    pub fn subset<C>(&mut self, chars: C) -> Result<Vec<u8>>
    where
        C: IntoIterator<Item = char>,
    {
        let mut glyphs = std::collections::HashSet::new();
        glyphs.insert(0);

        for ch in chars {
            if let Some(index) = self.glyph_index(ch) {
                glyphs.insert(index);
            }
        }

        let glyphs = glyphs.into_iter().collect::<Vec<_>>();

        Ok(subset(&self.inner.font_table_provider, &glyphs)?)
    }

    /// Get the units per em value of the font.
    fn em(&self) -> u16 {
        self.head.units_per_em
    }

    /// Get the range of a specific axis in the font.
    fn axis(&self, tag: u32) -> Option<(Fixed, Fixed)> {
        self.inner
            .variation_axes()
            .ok()?
            .into_iter()
            .find(|rec| rec.axis_tag == tag)
            .map(|rec| (rec.min_value, rec.max_value))
    }

    /// Get the glyph index for a specific character.
    fn glyph_index(&mut self, ch: char) -> Option<u16> {
        let index = self
            .inner
            .lookup_glyph_index(ch, MatchingPresentation::NotRequired, None)
            .0;
        if index == 0 { None } else { Some(index) }
    }
}
