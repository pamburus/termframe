// std imports
use std::{
    include_str,
    path::{Path, PathBuf},
    sync::LazyLock,
};

// third-party imports
use anyhow::Result;
use config::{Config, File, FileFormat};
use csscolorparser::Color;
use serde::Deserialize;

// local imports
use crate::appdirs::AppDirs;

// ---

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    pub terminal: Terminal,
    pub font: Font,
    pub line_height: f32,
    pub precision: u8,
    pub theme: String,
    pub fonts: Fonts,
    pub embed_fonts: bool,
    pub faint_opacity: f32,
    pub bold_is_bright: bool,
    pub padding: PaddingOption,
    pub stroke: f32,
    pub window: Window,
}

impl Settings {
    pub fn load<I>(sources: I) -> Result<Self>
    where
        I: IntoIterator<Item = Source>,
    {
        let mut builder =
            Config::builder().add_source(File::from_str(DEFAULT_SETTINGS_RAW, FileFormat::Yaml));

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

        Ok(builder.build()?.try_deserialize()?)
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

// ---

#[derive(Debug, Deserialize, Clone)]
pub struct Window {
    pub enabled: bool,
    pub margin: PaddingOption,
    pub border: WindowBorder,
    pub header: WindowHeader,
    pub buttons: WindowButtons,
    pub shadow: WindowShadow,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowBorder {
    pub color1: Color,
    pub color2: Color,
    pub width: f32,
    pub radius: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowHeader {
    pub color: Color,
    pub height: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowButtons {
    pub radius: f32,
    pub spacing: f32,
    pub close: WindowButton,
    pub minimize: WindowButton,
    pub maximize: WindowButton,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowButton {
    pub color: Color,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowShadow {
    pub enabled: bool,
    pub color: Color,
    pub x: f32,
    pub y: f32,
    pub blur: f32,
}

// ---

pub type Fonts = Vec<FontFace>;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct FontFace {
    pub family: String,
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Terminal {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Font {
    pub family: String,
    pub size: f32,
    pub weights: FontWeights,
}

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

impl ToString for FontWeight {
    fn to_string(&self) -> String {
        match self {
            Self::Normal => "normal".to_string(),
            Self::Bold => "bold".to_string(),
            Self::Fixed(weight) => weight.to_string(),
        }
    }
}

// ---

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum PaddingOption {
    Uniform(f32),
    Symmetric { vertical: f32, horizontal: f32 },
    Asymmetric(Padding),
}

impl PaddingOption {
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

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub struct Padding {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

// ---

static DEFAULT_SETTINGS_RAW: &str = include_str!("assets/config.yaml");
static DEFAULT_SETTINGS: LazyLock<Settings> =
    LazyLock::new(|| Settings::load([Source::string("", FileFormat::Yaml)]).unwrap());

pub const APP_NAME: &str = "termshot";

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

// ---

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

    pub fn no_default(mut self, val: bool) -> Self {
        self.no_default = val;
        self
    }

    pub fn load(self) -> Result<Settings> {
        if self.no_default {
            Settings::load(self.custom())
        } else {
            Settings::load(self.system().chain(self.user()).chain(self.custom()))
        }
    }

    fn system(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| dirs.system_config_dirs.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|dir| SourceFile::new(&Self::config(&dir)).required(false).into())
    }

    fn user(&self) -> impl Iterator<Item = Source> {
        self.dirs
            .as_ref()
            .map(|dirs| {
                SourceFile::new(&Self::config(&dirs.config_dir))
                    .required(false)
                    .into()
            })
            .into_iter()
    }

    fn custom<'a>(&'a self) -> impl Iterator<Item = Source> + 'a {
        self.paths
            .iter()
            .map(|path| SourceFile::new(path).required(true).into())
    }

    fn config(dir: &Path) -> PathBuf {
        dir.join("config")
    }
}

// ---

/// Get the application platform-specific directories.
pub fn app_dirs() -> Option<AppDirs> {
    AppDirs::new(APP_NAME)
}

// ---

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

// ---

pub enum Source {
    File(SourceFile),
    String(String, FileFormat),
}

impl Source {
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

// ---

pub struct SourceFile {
    filename: PathBuf,
    required: bool,
}

impl SourceFile {
    pub fn new<P>(filename: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            filename: filename.as_ref().into(),
            required: true,
        }
    }

    pub fn required(self, required: bool) -> Self {
        Self { required, ..self }
    }
}

// ---

pub trait Patch {
    fn patch(&self, settings: Settings) -> Settings;
}
