// std imports
use std::{
    io,
    num::{ParseFloatError, ParseIntError, TryFromIntError},
    path::{Path, PathBuf},
};

// third-party imports
use cached::{DiskCacheError, stores::DiskCacheBuildError};
use config::ConfigError;
use itertools::Itertools;
use nu_ansi_term::Color;
use thiserror::Error;

use crate::suggest::Suggestions;

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("failed to load configuration: {0}")]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error("failed to open file {}: {source}", .name.hl())]
    FailedToOpenFile { name: PathBuf, source: io::Error },
    #[error("unknown theme {}, did you mean any of {}?", .name.hl(), .suggestions.hl())]
    UnknownTheme {
        name: String,
        suggestions: Suggestions,
    },
    #[error("failed to parse utf-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("failed to construct utf-8 string from bytes: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("failed to parse yaml: {0}")]
    YamlError(#[from] serde_yml::Error),
    #[error("failed to parse toml: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("failed to parse json: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    DiskCacheError(#[from] DiskCacheError),
    #[error(transparent)]
    DiskCacheBuildError(#[from] DiskCacheBuildError),
    #[error("failed to detect application directories")]
    FailedToDetectAppDirs,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    pub fn log(&self) {
        eprintln!("{} {}", Color::LightRed.bold().paint("error:"), self);
    }
}

/// Result is an alias for standard result with bound Error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub const HILITE: Color = Color::Yellow;

trait Highlight {
    fn hl(&self) -> String;
}

impl<S: AsRef<str>> Highlight for S {
    fn hl(&self) -> String {
        HILITE.paint(format!("{:?}", self.as_ref())).to_string()
    }
}

impl Highlight for Path {
    fn hl(&self) -> String {
        HILITE.paint(self.to_string_lossy()).to_string()
    }
}

impl Highlight for Suggestions {
    fn hl(&self) -> String {
        format!("[ {} ]", self.iter().map(|x| x.hl()).join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log() {
        let err = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        err.log();
    }
}
