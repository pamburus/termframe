// std imports
use std::{
    borrow::Cow,
    fmt, io,
    num::{ParseFloatError, ParseIntError, TryFromIntError},
};

// third-party imports
use config::ConfigError;
use owo_colors::OwoColorize;
use thiserror::Error;

use crate::{
    config::{theme, winstyle},
    xerr::{HighlightQuoted, Suggestions},
};

/// Result is an alias for standard result with bound Error type.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error is an error which may occur in the application.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Configuration loading error
    #[error("failed to load configuration: {0}")]
    Config(#[from] ConfigError),

    /// Infallible error
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),

    /// Theme error
    #[error(transparent)]
    Theme(#[from] theme::Error),

    /// Window style error
    #[error(transparent)]
    WindowStyle(#[from] winstyle::Error),

    /// UTF-8 parsing error
    #[error("failed to parse utf-8 string: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// UTF-8 construction error from bytes
    #[error("failed to construct utf-8 string from bytes: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    /// YAML parsing error
    #[error("failed to parse yaml: {0}")]
    Yaml(#[from] serde_yml::Error),

    /// TOML parsing error
    #[error("failed to parse toml: {0}")]
    Toml(#[from] toml::de::Error),

    /// JSON parsing error
    #[error("failed to parse json: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Integer conversion error
    #[error(transparent)]
    TryFromInt(#[from] TryFromIntError),

    /// Float parsing error
    #[error(transparent)]
    ParseFloat(#[from] ParseFloatError),

    /// Integer parsing error
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Provides tips based on the error type.
    fn tips<'a, A>(&'a self, app: &A) -> Tips<'a>
    where
        A: AppInfoProvider,
    {
        match self {
            Error::Theme(theme::Error::ThemeNotFound { suggestions, .. }) => {
                let did_you_mean = did_you_mean(suggestions);
                let usage = usage(app, UsageRequest::ListThemes)
                    .map(|usage| format!("run {usage} to list available themes"));
                Tips {
                    did_you_mean,
                    usage,
                }
            }
            Error::WindowStyle(winstyle::Error::WindowStyleNotFound { suggestions, .. }) => {
                let did_you_mean = did_you_mean(suggestions);
                let usage = usage(app, UsageRequest::ListWindowStyles)
                    .map(|usage| format!("run {usage} to list available window styles"));
                Tips {
                    did_you_mean,
                    usage,
                }
            }
            _ => Default::default(),
        }
    }

    /// Logs the error to stderr.
    pub fn log<A>(&self, app: &A)
    where
        A: AppInfoProvider,
    {
        self.log_to(&mut io::stderr(), app).ok();
    }

    /// Logs the error to a specified target.
    pub fn log_to<A, W>(&self, target: &mut W, app: &A) -> io::Result<()>
    where
        A: AppInfoProvider,
        W: std::io::Write,
    {
        writeln!(target, "{} {:#}", ERR_PREFIX.bright_red().bold(), self)?;
        write!(target, "{}", self.tips(app))?;
        Ok(())
    }
}

/// Tips struct containing suggestions and usage information.
#[derive(Debug, Default)]
struct Tips<'a> {
    did_you_mean: Option<DidYouMean<'a>>,
    usage: Option<String>,
}

impl std::fmt::Display for Tips<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let prefix = TIP_PREFIX.green();
        let prefix = prefix.bold();

        if let Some(did_you_mean) = &self.did_you_mean {
            writeln!(f, "{prefix} {did_you_mean}")?;
        }

        if let Some(usage) = &self.usage {
            writeln!(f, "{prefix} {usage}")?;
        }

        Ok(())
    }
}

/// Trait for providing application information.
pub trait AppInfoProvider {
    /// Returns the application name.
    fn app_name(&self) -> Cow<'static, str> {
        std::env::args()
            .nth(0)
            .map(Cow::Owned)
            .unwrap_or("<app>".into())
    }

    /// Provides usage suggestions based on the request.
    fn usage_suggestion(&self, _request: UsageRequest) -> Option<UsageResponse> {
        None
    }
}

/// Enum representing usage request types.
pub enum UsageRequest {
    ListThemes,
    ListWindowStyles,
}

/// Type alias for usage response.
pub type UsageResponse = (Cow<'static, str>, Cow<'static, str>);

/// Generates usage information based on the request.
fn usage<A: AppInfoProvider>(app: &A, request: UsageRequest) -> Option<String> {
    let (command, args) = app.usage_suggestion(request)?;
    let result = format!("{} {}", app.app_name(), command);
    let result = result.bold();
    if args.is_empty() {
        Some(result.to_string())
    } else {
        Some(format!("{} {}", result, args))
    }
}

/// Struct representing "Did You Mean" suggestions.
#[derive(Debug)]
pub struct DidYouMean<'a> {
    suggestions: &'a Suggestions,
}

impl fmt::Display for DidYouMean<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "did you mean ")?;
        for (i, suggestion) in self.suggestions.iter().enumerate() {
            if i > 0 {
                write!(f, " or ")?;
            }
            write!(f, "{}", suggestion.hlq())?;
        }
        write!(f, "?")
    }
}

/// Generates "Did You Mean" suggestions.
fn did_you_mean(suggestions: &Suggestions) -> Option<DidYouMean> {
    if suggestions.is_empty() {
        return None;
    }

    Some(DidYouMean { suggestions })
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
        let err = Error::Io(std::io::Error::other("test"));
        let mut buf = Vec::new();
        err.log_to(&mut buf, &TestAppInfo).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "\u{1b}[1m\u{1b}[91merror:\u{1b}[39m\u{1b}[0m test\n"
        );
    }
}
