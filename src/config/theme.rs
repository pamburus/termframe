// std imports
use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, LazyLock},
};

// third-party imports
use csscolorparser::Color;
use enumset::{EnumSet, EnumSetType};
use rust_embed::RustEmbed;
use serde::Deserialize;
use strum::Display;
use thiserror::Error;

// local imports
use super::{
    load::{self, Categorize, ErrorCategory, Load},
    mode::Mode,
};
use crate::xerr::{HighlightQuoted, Suggestions};

// ---

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown theme {}", .name.hlq())]
    ThemeNotFound {
        name: Arc<str>,
        suggestions: Suggestions,
    },
    #[error("theme file {} not found", .path.hlq())]
    ThemeFileNotFound { path: PathBuf },
    #[error("invalid theme file path {}", .path.hlq())]
    InvalidThemeFilePath { path: PathBuf },
    #[error("invalid tag {value}", value=.value.hlq())]
    InvalidTag {
        value: Arc<str>,
        suggestions: Suggestions,
    },
    #[error("failed to list themes: {source}")]
    FailedToListThemes { source: io::Error },
    #[error("failed to load theme {name}: {source}", name=.name.hlq())]
    Io { name: Arc<str>, source: io::Error },
    #[error("failed to parse theme {name}: {source}", name=.name.hlq())]
    FailedToParseTheme {
        name: Arc<str>,
        source: load::ParseError,
    },
}

impl From<load::Error> for Error {
    fn from(err: load::Error) -> Self {
        match err {
            load::Error::ItemNotFound {
                name, suggestions, ..
            } => Self::ThemeNotFound { name, suggestions },
            load::Error::FileNotFound { path } => Self::ThemeFileNotFound { path },
            load::Error::InvalidFilePath { path } => Self::InvalidThemeFilePath { path },
            load::Error::FailedToListItems { source, .. } => Self::FailedToListThemes { source },
            load::Error::Io { name, source, .. } => Self::Io { name, source },
            load::Error::Parse { name, source, .. } => Self::FailedToParseTheme { name, source },
        }
    }
}

impl Categorize for Error {
    fn category(&self) -> ErrorCategory {
        match self {
            Error::ThemeNotFound { .. } => ErrorCategory::ItemNotFound,
            _ => ErrorCategory::Other,
        }
    }
}

// ---

#[derive(Debug, Ord, PartialOrd, Hash, Deserialize, EnumSetType, Display)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum Tag {
    Dark,
    Light,
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Error::InvalidTag {
            value: s.into(),
            suggestions: Suggestions::new(s, EnumSet::<Tag>::all().iter().map(|v| v.to_string())),
        })
    }
}

// ---

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ThemeConfig {
    #[serde(deserialize_with = "enumset_serde::deserialize")]
    pub tags: EnumSet<Tag>,
    pub theme: Theme,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum Theme {
    Fixed(Fixed),
    Adaptive(Adaptive),
}

impl Theme {
    pub fn resolve(&self, mode: Mode) -> &Colors {
        match self {
            Theme::Fixed(fixed) => &fixed.colors,
            Theme::Adaptive(dynamic) => match mode {
                Mode::Dark => &dynamic.modes.dark.colors,
                Mode::Light => &dynamic.modes.light.colors,
            },
        }
    }
}

impl Load for ThemeConfig {
    type Assets = Assets;
    type Error = Error;

    fn category() -> &'static str {
        "themes"
    }

    fn dir_name() -> &'static str {
        Self::category()
    }

    fn resolve_embedded_name_alias(alias: &str) -> &str {
        ALIAS_MAP.name(alias).unwrap_or(alias)
    }

    fn preferred_embedded_name_alias(name: &str) -> &str {
        ALIAS_MAP.alias(name).unwrap_or(name)
    }

    fn is_not_found_error(err: &Error) -> bool {
        matches!(err, Error::ThemeNotFound { .. })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Fixed {
    pub colors: Colors,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Adaptive {
    pub modes: Modes,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Modes {
    pub dark: Fixed,
    pub light: Fixed,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Colors {
    pub background: Color,
    pub foreground: Color,
    pub bright_foreground: Option<Color>,
    pub palette: Palette,
}

pub type Palette = HashMap<PaletteIndex, Color>;

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum PaletteIndex {
    U8(u8),
    String(String),
}

impl PaletteIndex {
    pub fn resolve(&self) -> Option<u8> {
        match self {
            Self::U8(value) => Some(*value),
            Self::String(value) => value.parse().ok(),
        }
    }
}

// ---

struct AliasMap {
    a2n: HashMap<String, String>,
    n2a: HashMap<String, String>,
}

impl AliasMap {
    fn load() -> Self {
        let asset = Assets::get(".aliases.json").unwrap();
        Self::new(serde_json::from_slice(&asset.data).unwrap())
    }

    fn new(a2n: HashMap<String, String>) -> Self {
        let mut n2a = HashMap::new();
        for (alias, name) in a2n.iter() {
            n2a.insert(name.clone(), alias.clone());
        }

        Self { a2n, n2a }
    }

    fn alias(&self, name: &str) -> Option<&str> {
        self.n2a.get(name).map(|s| s.as_str())
    }

    fn name(&self, alias: &str) -> Option<&str> {
        self.a2n.get(alias).map(|s| s.as_str())
    }
}

// ---

#[derive(RustEmbed)]
#[folder = "assets/themes/"]
pub struct Assets;

static ALIAS_MAP: LazyLock<AliasMap> = LazyLock::new(AliasMap::load);
