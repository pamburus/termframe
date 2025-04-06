// std imports
use std::{
    collections::HashMap,
    io,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

// third-party imports
use rust_embed::RustEmbed;
use serde::de::DeserializeOwned;
use serde_json as json;
use serde_yml as yaml;
use strum::{Display, EnumIter, IntoEnumIterator};
use thiserror::Error;

// local imports
use crate::xerr::{Highlight, Suggestions};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Custom error type for handling various errors in the application.
#[derive(Error, Debug)]
pub enum Error {
    /// Error for unknown item.
    #[error("unknown item {name} in {category}", name=.name.hl(), category=.category.hl())]
    ItemNotFound {
        name: Arc<str>,
        category: &'static str,
        suggestions: Suggestions,
    },

    /// Error for invalid file path.
    #[error("invalid file path {path}", path=.path.hl())]
    InvalidFilePath { path: PathBuf },

    /// Error for file not found.
    #[error("file {path} not found", path=.path.hl())]
    FileNotFound { path: PathBuf },

    /// Error for failing to list items in a category.
    #[error("failed to list items in {category}: {source}", category=.category.hl())]
    FailedToListItems {
        category: &'static str,
        source: io::Error,
    },

    /// Error for I/O operations.
    #[error("I/O error loading {name} in {category}: {source}", name=.name.hl())]
    Io {
        name: Arc<str>,
        category: &'static str,
        source: io::Error,
    },

    /// Error for parsing items.
    #[error("failed to parse item {name} in {category}: {source}", name=.name.hl())]
    Parse {
        name: Arc<str>,
        category: &'static str,
        source: ParseError,
    },
}

/// Custom error type for handling parsing errors.
#[derive(Error, Debug)]
pub enum ParseError {
    /// Error for parsing YAML.
    #[error("failed to parse yaml: {0}")]
    Yaml(#[from] serde_yml::Error),

    /// Error for parsing TOML.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),

    /// Error for parsing JSON.
    #[error("failed to parse json: {0}")]
    Json(#[from] serde_json::Error),

    /// Error for parsing UTF-8 strings.
    #[error("failed to parse utf-8 string: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// Trait for categorizing errors.
pub trait Categorize {
    fn category(&self) -> ErrorCategory;
}

/// Enum for error categories.
pub enum ErrorCategory {
    ItemNotFound,
    Other,
}

/// Trait for loading assets.
pub trait Load {
    type Assets: RustEmbed;
    type Error: From<Error> + Categorize;

    /// Load an asset by name.
    fn load(name: &str) -> Result<Self, Self::Error>
    where
        Self: DeserializeOwned + Sized,
    {
        match Self::load_from(&Self::dir(), name) {
            Ok(r) => Ok(r),
            Err(e) => match e.category() {
                ErrorCategory::ItemNotFound => Self::embedded(name),
                _ => Err(e),
            },
        }
    }

    /// Load an embedded asset by name.
    fn embedded(name: &str) -> Result<Self, Self::Error>
    where
        Self: DeserializeOwned,
    {
        let name = Self::resolve_embedded_name_alias(name);
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            if let Some(file) = Self::Assets::get(&filename) {
                return Ok(Self::from_buf(file.data.as_ref(), format).map_err(|e| {
                    Error::Parse {
                        name: name.into(),
                        category: Self::category(),
                        source: e,
                    }
                })?);
            }
        }

        let suggestions = Suggestions::new(name, Self::embedded_names());

        Err(Self::Error::from(Error::ItemNotFound {
            name: name.into(),
            category: Self::category(),
            suggestions,
        }))
    }

    /// List all available assets.
    fn list() -> Result<HashMap<String, ItemInfo>, Self::Error> {
        let mut result = HashMap::new();

        for name in Self::embedded_names() {
            result.insert(name, Origin::Stock.into());
        }

        if let Ok(names) = Self::custom_names() {
            for name in names {
                result.insert(name?, Origin::Custom.into());
            }
        }

        Ok(result)
    }

    /// Deserialize an asset from a byte buffer.
    fn from_buf(data: &[u8], format: Format) -> Result<Self, ParseError>
    where
        Self: DeserializeOwned + Sized,
    {
        let s = std::str::from_utf8(data)?;
        match format {
            Format::Yaml => Ok(yaml::from_str(s)?),
            Format::Toml => Ok(toml::from_str(s)?),
            Format::Json => Ok(json::from_str(s)?),
        }
    }

    /// Load an asset from a directory.
    fn load_from(dir: &Path, name: &str) -> Result<Self, Self::Error>
    where
        Self: DeserializeOwned + Sized,
    {
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            let path = PathBuf::from(&filename);
            let path = if matches!(
                path.components().next(),
                Some(Component::ParentDir | Component::CurDir)
            ) {
                path
            } else {
                dir.join(&filename)
            };
            match std::fs::read(&path) {
                Ok(data) => {
                    return Ok(Self::from_buf(&data, format).map_err(|e| Error::Parse {
                        name: name.into(),
                        category: Self::category(),
                        source: e,
                    })?);
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => continue,
                    _ => {
                        return Err(Error::Io {
                            name: name.into(),
                            category: Self::category(),
                            source: e,
                        }
                        .into());
                    }
                },
            }
        }

        Err(Error::ItemNotFound {
            name: name.into(),
            category: Self::category(),
            suggestions: Suggestions::none(),
        }
        .into())
    }

    /// Load an asset from a hybrid path or name.
    fn load_hybrid(theme_or_path: &str) -> Result<Self, Self::Error>
    where
        Self: DeserializeOwned + Sized,
    {
        let theme = theme_or_path;
        let path = PathBuf::from(theme);
        match (path.parent(), path.file_name().and_then(|x| x.to_str())) {
            (Some(dir), _) if dir == Path::new("") => Self::load(theme),
            (Some(dir), Some(filename)) => match Self::load_from(dir, filename) {
                Ok(cfg) => Ok(cfg),
                Err(err) if Self::is_not_found_error(&err) => {
                    Err(Error::FileNotFound { path }.into())
                }
                Err(err) => Err(err),
            },
            _ => Err(Error::InvalidFilePath { path }.into()),
        }
    }

    /// Generate a filename for an asset.
    fn filename(name: &str, format: Format) -> String {
        if Self::strip_extension(name, format).is_some() {
            return name.to_string();
        }

        format!("{}.{}", name, format.extension())
    }

    /// Get the directory for custom assets.
    fn dir() -> PathBuf {
        super::app_dirs()
            .map(|app_dirs| app_dirs.config_dir.join(Self::dir_name()))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn category() -> &'static str;
    fn dir_name() -> &'static str;
    fn is_not_found_error(err: &Self::Error) -> bool;

    /// Resolve an alias for an embedded asset name.
    fn resolve_embedded_name_alias(name_or_alias: &str) -> &str {
        name_or_alias
    }

    /// Get the preferred alias for an embedded asset name.
    fn preferred_embedded_name_alias(name: &str) -> &str {
        name
    }

    /// Get the names of all embedded assets.
    fn embedded_names() -> impl IntoIterator<Item = String> {
        Self::Assets::iter().filter_map(|a| {
            if a.starts_with('.') {
                return None;
            }
            Self::strip_known_extension(&a)
                .map(|n| Self::preferred_embedded_name_alias(n).to_string())
        })
    }

    /// Get the names of all custom assets.
    fn custom_names() -> Result<impl IntoIterator<Item = Result<String>>> {
        let path = Self::dir();
        let dir = Path::new(&path);
        Ok(dir
            .read_dir()
            .map_err(|e| Error::FailedToListItems {
                category: Self::category(),
                source: e,
            })?
            .map(|item| {
                Ok(item
                    .map_err(|e| Error::FailedToListItems {
                        category: Self::category(),
                        source: e,
                    })?
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|a| Self::strip_known_extension(a).map(|n| n.to_string())))
            })
            .filter_map(|x| x.transpose()))
    }

    /// Strip the extension from a filename.
    fn strip_extension(filename: &str, format: Format) -> Option<&str> {
        filename
            .strip_suffix(format.extension())
            .and_then(|r| r.strip_suffix("."))
    }

    /// Strip any known extension from a filename.
    fn strip_known_extension(filename: &str) -> Option<&str> {
        for format in Format::iter() {
            if let Some(name) = Self::strip_extension(filename, format) {
                return Some(name);
            }
        }
        None
    }
}

/// Enum for supported file formats.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, EnumIter)]
pub enum Format {
    Yaml,
    Toml,
    Json,
}

impl Format {
    /// Get the file extension for the format.
    pub fn extension(&self) -> &str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
        }
    }
}

/// Struct for storing item information.
#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub origin: Origin,
}

impl From<Origin> for ItemInfo {
    fn from(origin: Origin) -> Self {
        Self { origin }
    }
}

/// Enum for the origin of an item.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Origin {
    Stock,
    Custom,
}
