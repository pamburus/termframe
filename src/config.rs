// std imports
use std::{
    collections::HashMap,
    fmt, include_str,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

// third-party imports
use anyhow::{Context, Result};
use config::{Config, File, FileFormat};
use serde::{Deserialize, Deserializer};

// local imports
use crate::appdirs::AppDirs;

pub mod load;
pub mod mode;
pub mod theme;
pub mod types;
pub mod winstyle;

pub use load::Load;
pub use types::Number;

pub const APP_NAME: &str = "termframe";

static DEFAULT_SETTINGS_RAW: &str = include_str!("../assets/config.toml");
const DEFAULT_SETTINGS_FORMAT: FileFormat = FileFormat::Toml;
static DEFAULT_SETTINGS: LazyLock<Settings> =
    LazyLock::new(|| Settings::load([Source::string("", DEFAULT_SETTINGS_FORMAT)]).unwrap());

/// Get the default settings.
#[allow(dead_code)]
pub fn default() -> &'static Settings {
    Default::default()
}

/// Load settings from the given file.
pub fn at<I, P>(paths: I) -> Loader
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    Loader::new(paths.into_iter().map(|path| path.as_ref().into()).collect())
}

/// Load settings from the default configuration file per platform.
#[allow(dead_code)]
pub fn load() -> Result<Settings> {
    Loader::new(Vec::new()).load()
}

/// Get the application platform-specific directories.
pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(APP_NAME)
}

pub mod global {
    use super::*;
    use std::sync::Mutex;

    static PENDING: Mutex<Option<Settings>> = Mutex::new(None);
    static RESOLVED: LazyLock<Settings> =
        LazyLock::new(|| PENDING.lock().unwrap().take().unwrap_or_default());

    /// Call initialize before any calls to get otherwise it will have no effect.
    pub fn initialize(cfg: Settings) {
        *PENDING.lock().unwrap() = Some(cfg);
    }

    /// Get the resolved config.
    /// If initialized was called before, then a clone of that config will be returned.
    /// Otherwise, the default config will be returned.
    pub fn get() -> &'static Settings {
        &RESOLVED
    }
}

/// Settings structure containing various configuration options.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    pub terminal: Terminal,
    pub mode: mode::ModeSetting,
    pub theme: ThemeSetting,
    pub font: Font,
    pub padding: PaddingOption,
    pub window: Window,
    pub env: HashMap<String, String>,
    pub rendering: Rendering,
    pub fonts: Fonts,
}

impl Settings {
    /// Load settings from the provided sources.
    pub fn load<I>(sources: I) -> Result<Self>
    where
        I: IntoIterator<Item = Source>,
    {
        let mut builder = Config::builder().add_source(File::from_str(
            DEFAULT_SETTINGS_RAW,
            DEFAULT_SETTINGS_FORMAT,
        ));

        for source in sources {
            builder = match source {
                Source::File(SourceFile { filename, required }) => {
                    log::debug!(
                        "added configuration file {} search path: {}",
                        if required { "required" } else { "optional" },
                        filename.display(),
                    );
                    builder.add_source(File::from(filename.as_path()).required(required))
                }
                Source::String(value, format) => builder.add_source(File::from_str(&value, format)),
            };
        }

        builder
            .build()?
            .try_deserialize()
            .context("failed to load config")
    }
}

impl Default for Settings {
    fn default() -> Self {
        DEFAULT_SETTINGS.clone()
    }
}

impl Default for &'static Settings {
    fn default() -> Self {
        &DEFAULT_SETTINGS
    }
}

/// Rendering settings structure.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Rendering {
    pub line_height: Number,
    pub faint_opacity: Number,
    pub bold_is_bright: bool,
    pub svg: Svg,
}

/// SVG settings structure.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Svg {
    pub stroke: Option<Number>,
    pub precision: u8,
    pub embed_fonts: bool,
    pub subset_fonts: bool,
    pub var_palette: bool,
}

/// Window settings structure.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Window {
    pub enabled: bool,
    pub shadow: bool,
    pub style: String,
    pub margin: Option<PaddingOption>,
}

/// Theme setting enumeration.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum ThemeSetting {
    Fixed(String),
    Adaptive { light: String, dark: String },
}

impl ThemeSetting {
    /// Resolve the theme based on the mode.
    pub fn resolve(&self, mode: mode::Mode) -> &str {
        match self {
            Self::Fixed(theme) => theme,
            Self::Adaptive { light, dark } => match mode {
                mode::Mode::Light => light,
                mode::Mode::Dark => dark,
            },
        }
    }
}

pub type Fonts = Vec<FontFace>;

/// Font face structure.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct FontFace {
    pub family: String,
    pub files: Vec<String>,
    pub fallback: Option<FontFaceFallback>,
}

/// Font face fallback structure.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct FontFaceFallback {
    pub family: String,
    pub files: Vec<String>,
}

/// Terminal settings structure.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "kebab-case")]
pub struct Terminal {
    pub width: Dimension<u16>,
    pub height: Dimension<u16>,
}

/// Font settings structure.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Font {
    pub family: FontFamilyOption,
    pub size: Number,
    pub weights: FontWeights,
}

/// Font family option enumeration.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum FontFamilyOption {
    Single(String),
    Multiple(Vec<String>),
}

impl FontFamilyOption {
    /// Get the primary font family.
    pub fn primary(&self) -> &str {
        match self {
            Self::Single(family) => family,
            Self::Multiple(families) => &families[0],
        }
    }

    /// Resolve the font family option to a vector of strings.
    pub fn resolve(&self) -> Vec<String> {
        match self {
            Self::Single(family) => vec![family.clone()],
            Self::Multiple(families) => families.clone(),
        }
    }

    /// Check if the font family option contains a specific family.
    pub fn contains(&self, family: &str) -> bool {
        match self {
            Self::Single(f) => f == family,
            Self::Multiple(f) => f.contains(&family.to_string()),
        }
    }
}

/// Font weights structure.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct FontWeights {
    pub normal: FontWeight,
    pub bold: FontWeight,
    pub faint: FontWeight,
}

impl Default for FontWeights {
    fn default() -> Self {
        Self {
            normal: FontWeight::Normal,
            bold: FontWeight::Bold,
            faint: FontWeight::Normal,
        }
    }
}

/// Font weight enumeration.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
    #[serde(untagged)]
    Fixed(u16),
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Bold => write!(f, "bold"),
            Self::Fixed(weight) => write!(f, "{weight}"),
        }
    }
}

/// Padding option enumeration.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum PaddingOption {
    Uniform(Number),
    Symmetric {
        vertical: Number,
        horizontal: Number,
    },
    Asymmetric(Padding),
}

impl PaddingOption {
    /// Resolve the padding option to a padding structure.
    pub fn resolve(&self) -> Padding {
        match self {
            Self::Uniform(value) => Padding {
                top: *value,
                bottom: *value,
                left: *value,
                right: *value,
            },
            Self::Symmetric {
                vertical,
                horizontal,
            } => Padding {
                top: *vertical,
                bottom: *vertical,
                left: *horizontal,
                right: *horizontal,
            },
            Self::Asymmetric(padding) => *padding,
        }
    }
}

impl Default for PaddingOption {
    fn default() -> Self {
        Self::Uniform(4.0.into())
    }
}

/// Padding structure.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Padding {
    pub top: Number,
    pub bottom: Number,
    pub left: Number,
    pub right: Number,
}

impl std::ops::MulAssign<f32> for Padding {
    /// Multiply all padding values by a scalar.
    fn mul_assign(&mut self, rhs: f32) {
        self.top *= rhs;
        self.bottom *= rhs;
        self.left *= rhs;
        self.right *= rhs;
    }
}

impl std::ops::Mul<f32> for Padding {
    type Output = Self;

    // Multiply all padding values by a scalar and return the result.
    fn mul(mut self, rhs: f32) -> Self {
        self *= rhs;
        self
    }
}

/// Loader structure for loading settings.
pub struct Loader {
    paths: Vec<PathBuf>,
    no_default: bool,
    dirs: Option<AppDirs>,
}

impl Loader {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            no_default: false,
            dirs: app_dirs(),
        }
    }

    /// Set whether to use the default settings.
    pub fn no_default(mut self, val: bool) -> Self {
        self.no_default = val;
        self
    }

    /// Load the settings.
    pub fn load(self) -> Result<Settings> {
        if self.no_default {
            Settings::load(self.custom())
        } else {
            Settings::load(self.system().chain(self.user()).chain(self.custom()))
        }
    }

    /// Get system configuration sources.
    fn system(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| dirs.system_config_dirs.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|dir| SourceFile::new(Self::config(&dir)).required(false).into())
    }

    /// Get user configuration sources.
    fn user(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| {
                SourceFile::new(Self::config(&dirs.config_dir))
                    .required(false)
                    .into()
            })
            .into_iter()
    }

    /// Get custom configuration sources.
    fn custom(&self) -> impl Iterator<Item = Source> {
        self.paths
            .iter()
            .map(|path| SourceFile::new(path).required(true).into())
    }

    /// Get the configuration path for a directory.
    fn config(dir: &Path) -> PathBuf {
        dir.join("config")
    }
}

/// Source enumeration for configuration sources.
#[derive(Debug, Clone)]
pub enum Source {
    File(SourceFile),
    String(String, FileFormat),
}

impl Source {
    /// Create a new string source.
    pub fn string<S>(value: S, format: FileFormat) -> Self
    where
        S: Into<String>,
    {
        Self::String(value.into(), format)
    }
}

impl From<SourceFile> for Source {
    fn from(file: SourceFile) -> Self {
        Self::File(file)
    }
}

/// Source file structure for configuration files.
#[derive(Debug, Clone)]
pub struct SourceFile {
    filename: PathBuf,
    required: bool,
}

impl SourceFile {
    /// Create a new source file.
    pub fn new<P>(filename: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            filename: filename.as_ref().into(),
            required: true,
        }
    }

    /// Set whether the source file is required.
    pub fn required(self, required: bool) -> Self {
        Self { required, ..self }
    }
}

/// Trait for patching settings.
pub trait Patch {
    fn patch(&self, settings: Settings) -> Settings;
}

/// Dimension enumeration supporting fixed values, auto-sizing, and range constraints.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Dimension<T> {
    Auto,
    Limited { min: Option<T>, max: Option<T> },
    Fixed(T),
}

impl<T> From<T> for Dimension<T> {
    fn from(value: T) -> Self {
        Self::Fixed(value)
    }
}

impl<'de, T> Deserialize<'de> for Dimension<T>
where
    T: Deserialize<'de> + FromStr + Copy,
    T::Err: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_dimension(deserializer)
    }
}

impl<T> Dimension<T>
where
    T: PartialOrd + Copy,
{
    /// Get the minimum value of the dimension if it exists.
    pub fn min(&self) -> Option<T> {
        match self {
            Self::Auto => None,
            Self::Limited { min, .. } => *min,
            Self::Fixed(value) => Some(*value),
        }
    }

    /// Get the maximum value of the dimension if it exists.
    pub fn max(&self) -> Option<T> {
        match self {
            Self::Auto => None,
            Self::Limited { max, .. } => *max,
            Self::Fixed(value) => Some(*value),
        }
    }

    /// Resolve the dimension to a fixed value based on the provided default.
    pub fn resolve(&self, default: T) -> T {
        match self {
            Self::Auto => default,
            Self::Limited { min, max } => {
                let mut value = default;
                if let Some(min) = min {
                    if value < *min {
                        value = *min;
                    }
                }
                if let Some(max) = max {
                    if value > *max {
                        value = *max;
                    }
                }
                value
            }
            Self::Fixed(value) => *value,
        }
    }
}

/// Range type for parsing range syntax like "80..", "..120", "80..120"
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct PartialRange<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> FromStr for PartialRange<T>
where
    T: FromStr + Copy,
{
    type Err = RangeParseError<T>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some(dot_pos) = s.find("..") else {
            return Err(RangeParseError::MissingDots);
        };

        if s[dot_pos + 2..].contains("..") {
            return Err(RangeParseError::InvalidFormat);
        }

        let (min_str, max_str) = s.split_at(dot_pos);
        let max_str = &max_str[2..]; // Skip the ".."

        let min = if min_str.is_empty() {
            None
        } else {
            Some(
                min_str
                    .parse::<T>()
                    .map_err(RangeParseError::BoundParseError)?,
            )
        };

        let max = if max_str.is_empty() {
            None
        } else {
            Some(
                max_str
                    .parse::<T>()
                    .map_err(RangeParseError::BoundParseError)?,
            )
        };

        Ok(PartialRange { min, max })
    }
}

pub enum RangeParseError<T>
where
    T: FromStr + Copy,
{
    MissingDots,
    InvalidFormat,
    BoundParseError(T::Err),
}

impl<T> std::fmt::Display for RangeParseError<T>
where
    T: FromStr + Copy,
    T::Err: std::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeParseError::MissingDots => write!(f, "expected range syntax with '..'"),
            RangeParseError::InvalidFormat => write!(f, "invalid range format"),
            RangeParseError::BoundParseError(e) => write!(f, "bound parse error: {}", e),
        }
    }
}

impl<'de, T> Deserialize<'de> for PartialRange<T>
where
    T: FromStr + Copy,
    T::Err: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        Self::from_str(s).map_err(serde::de::Error::custom)
    }
}

/// Custom deserializer for Dimension that supports range syntax
fn deserialize_dimension<'de, D, T>(deserializer: D) -> Result<Dimension<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + FromStr + Copy,
    T::Err: std::fmt::Display,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    #[serde(untagged)]
    enum DimensionInternal<T> {
        Auto,
        Limited { min: Option<T>, max: Option<T> },
        Fixed(T),
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Helper<T>
    where
        T: FromStr + Copy,
        T::Err: std::fmt::Display,
    {
        Range(PartialRange<T>),
        Original(DimensionInternal<T>),
    }

    match Helper::deserialize(deserializer)? {
        Helper::Range(range) => Ok(Dimension::Limited {
            min: range.min,
            max: range.max,
        }),
        Helper::Original(dim) => match dim {
            DimensionInternal::Auto => Ok(Dimension::Auto),
            DimensionInternal::Limited { min, max } => Ok(Dimension::Limited { min, max }),
            DimensionInternal::Fixed(val) => Ok(Dimension::Fixed(val)),
        },
    }
}
