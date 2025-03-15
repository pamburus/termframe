// std imports
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

// third-party imports
use rust_embed::RustEmbed;
use serde::de::DeserializeOwned;
use serde_json as json;
use serde_yml as yaml;
use strum::{EnumIter, IntoEnumIterator};

// local imports
use super::Result;

// ---

pub trait Load {
    type Assets: RustEmbed;

    fn load(name: &str) -> Result<Option<Self>>
    where
        Self: DeserializeOwned + Sized,
    {
        if let Some(r) = Self::load_from(&Self::dir(), name)? {
            return Ok(Some(r));
        }

        Self::embedded(name)
    }

    fn embedded(name: &str) -> Result<Option<Self>>
    where
        Self: DeserializeOwned,
    {
        let name = Self::resolve_embedded_name_alias(name);
        for format in Format::iter() {
            let filename = Self::filename(name, format);
            if let Some(file) = Self::Assets::get(&filename) {
                return Ok(Some(Self::from_buf(file.data.as_ref(), format)?));
            }
        }

        Ok(None)
    }

    fn list() -> Result<HashMap<String, ItemInfo>> {
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

    fn from_buf(data: &[u8], format: Format) -> Result<Self>
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

    fn load_from(dir: &PathBuf, name: &str) -> Result<Option<Self>>
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
                    return Ok(Some(Self::from_buf(&data, format)?));
                }
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => continue,
                    _ => return Err(e.into()),
                },
            }
        }

        Ok(None)
    }

    fn filename(name: &str, format: Format) -> String {
        if Self::strip_extension(&name, format).is_some() {
            return name.to_string();
        }

        format!("{}.{}", name, format.extension())
    }

    fn dir() -> PathBuf {
        super::app_dirs()
            .map(|app_dirs| app_dirs.config_dir.join(Self::dir_name()))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn dir_name() -> &'static str;

    fn resolve_embedded_name_alias(name_or_alias: &str) -> &str {
        name_or_alias
    }

    fn preferred_embedded_name_alias(name: &str) -> &str {
        name
    }

    fn embedded_names() -> impl IntoIterator<Item = String> {
        Self::Assets::iter().filter_map(|a| {
            if a.starts_with('.') {
                return None;
            }
            Self::strip_known_extension(&a)
                .map(|n| Self::preferred_embedded_name_alias(n).to_string())
        })
    }

    fn custom_names() -> Result<impl IntoIterator<Item = Result<String>>> {
        let path = Self::dir();
        let dir = Path::new(&path);
        Ok(dir
            .read_dir()?
            .map(|item| {
                Ok(item?
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|a| Self::strip_known_extension(&a).map(|n| n.to_string())))
            })
            .filter_map(|x| x.transpose()))
    }

    fn strip_extension(filename: &str, format: Format) -> Option<&str> {
        filename
            .strip_suffix(format.extension())
            .and_then(|r| r.strip_suffix("."))
    }

    fn strip_known_extension(filename: &str) -> Option<&str> {
        for format in Format::iter() {
            if let Some(name) = Self::strip_extension(filename, format) {
                return Some(name);
            }
        }
        None
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, EnumIter)]
pub enum Format {
    Yaml,
    Toml,
    Json,
}

impl Format {
    pub fn extension(&self) -> &str {
        match self {
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
        }
    }
}

// ---

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub origin: Origin,
}

impl From<Origin> for ItemInfo {
    fn from(origin: Origin) -> Self {
        Self { origin }
    }
}

// ---

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Origin {
    Stock,
    Custom,
}
