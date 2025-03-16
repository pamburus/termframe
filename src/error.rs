// std imports
use std::{
    borrow::Cow,
    io,
    num::{ParseFloatError, ParseIntError, TryFromIntError},
    path::PathBuf,
};

// third-party imports
use cached::{DiskCacheError, stores::DiskCacheBuildError};
use config::ConfigError;
use itertools::Itertools;
use nu_ansi_term::Color;
use thiserror::Error;

use crate::{
    config::{theme, winstyle},
    xerr::{Highlight, Suggestions},
};

/// Result is an alias for standard result with bound Error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

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
    #[error(transparent)]
    Theme(#[from] theme::Error),
    #[error(transparent)]
    WindowStyle(#[from] winstyle::Error),
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
    fn tips<A>(&self, app: &A) -> Vec<String>
    where
        A: AppInfoProvider,
    {
        match self {
            Error::Theme(theme::Error::ThemeNotFound { suggestions, .. }) => {
                if let Some(s) = did_you_mean(suggestions) {
                    vec![s]
                } else if let Some(usage) = usage(app, UsageRequest::ListThemes) {
                    vec![format!("run {usage} to list themes")]
                } else {
                    Vec::new()
                }
            }
            Error::WindowStyle(winstyle::Error::WindowStyleNotFound { suggestions, .. }) => {
                if let Some(s) = did_you_mean(suggestions) {
                    vec![s]
                } else if let Some(usage) = usage(app, UsageRequest::ListWindowStyles) {
                    vec![format!("run {usage} to list window styles")]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }
}

impl Error {
    pub fn log<A>(&self, app: &A)
    where
        A: AppInfoProvider,
    {
        eprintln!("{} {:#}", Color::LightRed.bold().paint(ERR_PREFIX), self);
        for tip in self.tips(app) {
            eprintln!("{} {}", Color::Green.bold().paint(TIP_PREFIX), tip);
        }
    }
}

pub trait AppInfoProvider {
    fn app_name(&self) -> Cow<'static, str> {
        std::env::args()
            .nth(0)
            .map(Cow::Owned)
            .unwrap_or("<app>".into())
    }

    fn usage_suggestion(&self, _request: UsageRequest) -> Option<UsageResponse> {
        None
    }
}

pub enum UsageRequest {
    ListThemes,
    ListWindowStyles,
}

pub type UsageResponse = (Cow<'static, str>, Cow<'static, str>);

fn usage<A: AppInfoProvider>(app: &A, request: UsageRequest) -> Option<String> {
    let (command, args) = app.usage_suggestion(request)?;
    let result = Color::Default
        .bold()
        .paint(format!("{} {}", app.app_name(), command));
    if args.is_empty() {
        Some(result.to_string())
    } else {
        Some(format!("{} {}", result, args))
    }
}

fn did_you_mean(suggestions: &Suggestions) -> Option<String> {
    if suggestions.is_empty() {
        return None;
    }

    Some(format!(
        "did you mean {}?",
        suggestions.iter().map(|x| x.hl()).join(" or ")
    ))
}

const ERR_PREFIX: &str = "error:";
const TIP_PREFIX: &str = "  tip:";

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAppInfo;
    impl AppInfoProvider for TestAppInfo {}

    #[test]
    fn test_log() {
        let err = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        err.log(&TestAppInfo);
    }
}
