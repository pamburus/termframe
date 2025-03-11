// std imports
use std::path::PathBuf;

// third-party imports
use allsorts::{
    binary::read::{ReadScope, ReadScopeOwned},
    font_data::{DynamicFontTableProvider, FontData},
    tables::{HeadTable, os2::Os2},
};
use anyhow::anyhow;
use url::Url;

// ---

#[allow(dead_code)]
pub struct FontFile {
    location: Location,
    data: ReadScopeOwned,
}

pub type Result<T> = anyhow::Result<T>;

impl FontFile {
    pub fn load(location: Location) -> Result<Self> {
        match location {
            Location::File(path) => Self::load_file(path),
            Location::Url(url) => Self::load_url(url),
        }
    }

    pub fn load_file(path: PathBuf) -> Result<Self> {
        let bytes = std::fs::read(&path)?;
        Self::load_bytes(&bytes, Location::File(path))
    }

    pub fn load_url(url: Url) -> Result<Self> {
        match url.scheme() {
            "file" | "" => Self::load_file(url.path().into()),
            _ => {
                let bytes = ureq::get(url.as_ref()).call()?.body_mut().read_to_vec()?;
                Self::load_bytes(&bytes, Location::Url(url))
            }
        }
    }

    pub fn load_bytes(bytes: &[u8], location: Location) -> Result<Self> {
        let data = ReadScopeOwned::new(ReadScope::new(bytes));
        Ok(Self { location, data })
    }

    #[allow(dead_code)]
    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn font(&self) -> Result<Font> {
        let provider = self.data.scope().read::<FontData>()?.table_provider(0)?;
        let inner = allsorts::Font::new(provider)?;
        let Some(head) = inner.head_table()? else {
            return Err(anyhow!("No head table found in the font"));
        };
        let Some(os2) = inner.os2_table()? else {
            return Err(anyhow!("No os/2 table found in the font"));
        };
        Ok(Font { inner, head, os2 })
    }
}

#[derive(Debug, Clone)]
pub enum Location {
    File(PathBuf),
    Url(Url),
}

impl Location {
    pub fn auto<S: AsRef<str>>(s: S) -> Self {
        match Url::parse(s.as_ref()) {
            Ok(url) => Self::Url(url),
            Err(_) => Self::File(PathBuf::from(s.as_ref())),
        }
    }

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

#[allow(dead_code)]
pub struct Font<'a> {
    inner: allsorts::Font<DynamicFontTableProvider<'a>>,
    head: HeadTable,
    os2: Os2,
}

impl<'a> Font<'a> {
    pub fn width(&self) -> f32 {
        self.inner.hhea_table.advance_width_max as f32 / self.em() as f32
    }

    pub fn ascender(&self) -> f32 {
        self.inner.hhea_table.ascender as f32 / self.em() as f32
    }

    pub fn descender(&self) -> f32 {
        self.inner.hhea_table.descender as f32 / self.em() as f32
    }

    #[allow(dead_code)]
    pub fn line_gap(&self) -> f32 {
        self.inner.hhea_table.line_gap as f32 / self.em() as f32
    }

    #[allow(dead_code)]
    pub fn weight(&self) -> u16 {
        self.os2.us_weight_class
    }

    pub fn italic(&self) -> bool {
        self.head.is_italic()
    }

    pub fn bold(&self) -> bool {
        self.head.is_bold()
    }

    fn em(&self) -> u16 {
        self.head.units_per_em
    }
}
