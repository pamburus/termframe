// std imports
use std::fmt;

// third-party imports
use clap::{
    ArgAction, Args, Parser,
    builder::{Styles, styling::AnsiColor},
    value_parser,
};
use clap_complete::Shell;
use enumset_ext::convert::str::EnumSet;

// local imports
use crate::config::{self, FontFamilyOption, PaddingOption, Settings, ThemeSetting};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

/// Terminal output SVG screenshot tool.
#[derive(Parser)]
#[command(version, styles = STYLES, disable_help_flag = true)]
pub struct Opt {
    #[command(flatten)]
    pub bootstrap: BootstrapArgs,

    /// Width of the virtual terminal window.
    #[arg(long, short = 'W', default_value_t = cfg().terminal.width, overrides_with = "width", value_name = "COLUMNS")]
    pub width: u16,

    /// Height of the virtual terminal window.
    #[arg(long, short = 'H', default_value_t = cfg().terminal.height, overrides_with = "height", value_name = "LINES")]
    pub height: u16,

    /// Override padding for the inner text in font size units.
    #[arg(long, overrides_with = "padding", value_name = "EM")]
    pub padding: Option<f32>,

    /// Font family, multiple comma separated values can be provided.
    #[arg(long, value_parser = trim, num_args = 1.., value_delimiter = ',', overrides_with = "font_family", value_name = "NAME")]
    pub font_family: Vec<String>,

    /// Font size.
    #[arg(long, default_value_t = cfg().font.size.into(), overrides_with = "font_size", value_name = "SIZE")]
    pub font_size: f32,

    /// Normal font weight.
    #[arg(long, default_value_t = cfg().font.weights.normal.into(), overrides_with = "font_weight", value_name = "WEIGHT")]
    pub font_weight: FontWeight,

    /// Embed fonts, if possible [note: make sure the font license allows this type of redistribution].
    #[arg(long, num_args = 1, default_value_t = cfg().rendering.svg.embed_fonts, overrides_with = "embed_fonts", value_name = "ENABLED")]
    pub embed_fonts: bool,

    /// Subset fonts by removing unused characters [experimental, known to have compatibility issues].
    #[arg(long, num_args = 1, default_value_t = cfg().rendering.svg.subset_fonts, overrides_with = "subset_fonts", value_name = "ENABLED")]
    pub subset_fonts: bool,

    /// Use bright colors for bold text.
    #[arg(long, num_args = 1, default_value_t = cfg().rendering.bold_is_bright, overrides_with = "bold_is_bright", value_name = "ENABLED")]
    pub bold_is_bright: bool,

    /// Bold text font weight.
    #[arg(long, default_value_t = cfg().font.weights.bold.into(), overrides_with = "bold_font_weight", value_name = "WEIGHT")]
    pub bold_font_weight: FontWeight,

    /// Faint text opacity.
    #[arg(long, default_value_t = cfg().rendering.faint_opacity.into(), overrides_with = "faint_opacity", value_name = "0..1")]
    pub faint_opacity: f32,

    /// Faint text font weight.
    #[arg(long, default_value_t = cfg().font.weights.faint.into(), overrides_with = "faint_font_weight", value_name = "WEIGHT")]
    pub faint_font_weight: FontWeight,

    /// Line height, factor of the font size.
    #[arg(long, default_value_t = cfg().rendering.line_height.into(), overrides_with = "line_height", value_name = "FACTOR")]
    pub line_height: f32,

    /// Override dark or light mode.
    #[arg(long, value_enum, default_value_t = cfg().mode, overrides_with = "mode")]
    pub mode: config::mode::ModeSetting,

    /// Color theme.
    #[arg(long, overrides_with = "theme")]
    pub theme: Option<String>,

    /// Enable window.
    #[arg(long, num_args = 1, default_value_t = cfg().window.enabled, overrides_with = "window", value_name = "ENABLED")]
    pub window: bool,

    /// Enable window shadow.
    #[arg(long, num_args = 1, default_value_t = cfg().window.shadow, overrides_with = "window_shadow", value_name = "ENABLED")]
    pub window_shadow: bool,

    /// Override window margin, in pixels.
    #[arg(long, overrides_with = "window_margin", value_name = "PIXELS")]
    pub window_margin: Option<f32>,

    /// Window style.
    #[arg(long, overrides_with = "window_style", value_name = "NAME")]
    pub window_style: Option<String>,

    /// Window title.
    #[arg(long, overrides_with = "title", value_name = "TITLE")]
    pub title: Option<String>,

    /// Build palette using CSS variables for basic ANSI colors.
    #[arg(long, num_args = 1, default_value_t = cfg().rendering.svg.var_palette, overrides_with = "var_palette", value_name = "ENABLED")]
    pub var_palette: bool,

    /// Output file, by default prints to stdout.
    #[arg(
        long,
        short = 'o',
        default_value = "-",
        overrides_with = "output",
        value_name = "FILE"
    )]
    pub output: String,

    /// Timeout for the command to run, in seconds.
    #[arg(
        long,
        overrides_with = "timeout",
        default_value_t = 5,
        value_name = "SECONDS"
    )]
    pub timeout: u64,

    /// Print available themes optionally filtered by tags.
    #[arg(
        long,
        num_args=0..=1,
        value_name = "TAGS",
        require_equals = true,
        value_parser = ThemeTagSet::clap_parser(),
    )]
    pub list_themes: Option<Option<ThemeTagSet>>,

    /// Print available window styles and exit.
    #[arg(long)]
    pub list_window_styles: bool,

    /// Print configured fonts and exit, any font not listed here cannot be embedded and may not be properly rendered.
    #[arg(long)]
    pub list_fonts: bool,

    /// Print help and exit.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub help: bool,

    /// Print shell auto-completion script and exit.
    #[arg(long, value_parser = value_parser!(Shell), value_name = "SHELL")]
    pub shell_completions: Option<Shell>,

    /// Print man page and exit.
    #[arg(long)]
    pub man_page: bool,

    /// Command to run.
    pub command: Option<String>,

    /// Arguments provided to the command.
    #[arg(trailing_var_arg(true))]
    pub args: Vec<String>,
}

impl config::Patch for Opt {
    /// Applies the options from the command line arguments to the given settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The current settings to be patched.
    ///
    /// # Returns
    ///
    /// The patched settings.
    fn patch(&self, settings: Settings) -> Settings {
        let mut settings = settings;

        settings.terminal.width = self.width;
        settings.terminal.height = self.height;
        if !self.font_family.is_empty() {
            settings.font.family = FontFamilyOption::Multiple(self.font_family.clone());
        }
        settings.font.size = self.font_size.into();
        settings.font.weights.normal = self.font_weight.into();
        settings.font.weights.bold = self.bold_font_weight.into();
        settings.font.weights.faint = self.faint_font_weight.into();
        settings.rendering.svg.embed_fonts = self.embed_fonts;
        settings.rendering.svg.subset_fonts = self.subset_fonts;
        settings.rendering.svg.var_palette = self.var_palette;
        settings.rendering.faint_opacity = self.faint_opacity.into();
        settings.rendering.line_height = self.line_height.into();
        settings.rendering.bold_is_bright = self.bold_is_bright;
        if let Some(theme) = &self.theme {
            settings.theme = ThemeSetting::Fixed(theme.clone());
        }
        if let Some(padding) = self.padding {
            settings.padding = PaddingOption::Uniform(padding.into());
        }
        if let Some(style) = &self.window_style {
            settings.window.style = style.clone();
        }
        settings.window.enabled = self.window;
        settings.window.shadow = self.window_shadow;
        if let Some(margin) = self.window_margin {
            settings.window.margin = Some(PaddingOption::Uniform(margin.into()));
        }
        settings.mode = self.mode;

        settings
    }
}

#[derive(Args)]
pub struct BootstrapArgs {
    /// Configuration file path.
    #[arg(long, value_name = "FILE", env = "TERMFRAME_CONFIG", num_args = 1)]
    pub config: Vec<String>,
}

/// Terminal output snapshot tool.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
pub struct BootstrapOpt {
    #[command(flatten)]
    pub args: BootstrapArgs,
}

impl BootstrapOpt {
    /// Parses the command line arguments and returns a `BootstrapOpt` instance.
    ///
    /// # Returns
    ///
    /// A `BootstrapOpt` instance containing the parsed arguments.
    pub fn parse() -> Self {
        Self::parse_from(Self::args())
    }

    /// Retrieves the command line arguments.
    ///
    /// # Returns
    ///
    /// A vector of strings representing the command line arguments.
    pub fn args() -> Vec<String> {
        let mut args = wild::args();
        let Some(first) = args.next() else {
            return vec![];
        };

        let mut result = vec![first];
        let mut follow_up = false;

        for arg in args {
            match (arg.as_bytes(), follow_up) {
                (b"--", _) => {
                    break;
                }
                ([b'-', b'-', b'c', b'o', b'n', b'f', b'i', b'g', b'=', ..], _) => {
                    result.push(arg);
                    follow_up = false;
                }
                (b"--config", _) => {
                    result.push(arg);
                    follow_up = true;
                }
                ([b'-'], true) => {
                    result.push(arg);
                    follow_up = false;
                }
                ([b'-', ..], true) => {
                    follow_up = false;
                }
                (_, true) => {
                    result.push(arg);
                    follow_up = false;
                }
                _ => {}
            }
        }

        result
    }
}

pub type ThemeTagSet = EnumSet<config::theme::Tag>;

/// Font weight option.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Fixed(u16),
}

impl From<config::FontWeight> for FontWeight {
    fn from(weight: config::FontWeight) -> Self {
        match weight {
            config::FontWeight::Normal => Self::Normal,
            config::FontWeight::Bold => Self::Bold,
            config::FontWeight::Fixed(weight) => Self::Fixed(weight),
        }
    }
}

impl From<FontWeight> for config::FontWeight {
    fn from(weight: FontWeight) -> Self {
        match weight {
            FontWeight::Normal => Self::Normal,
            FontWeight::Bold => Self::Bold,
            FontWeight::Fixed(weight) => Self::Fixed(weight),
        }
    }
}

impl std::str::FromStr for FontWeight {
    type Err = String;

    /// Parses a string into a `FontWeight`.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse.
    ///
    /// # Returns
    ///
    /// A `Result` containing the parsed `FontWeight` or an error message.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            s => match s.parse() {
                Ok(weight) => Ok(Self::Fixed(weight)),
                Err(_) => Err(format!("Invalid font weight: {s}")),
            },
        }
    }
}

impl fmt::Display for FontWeight {
    /// Formats the `FontWeight` for display.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    ///
    /// # Returns
    ///
    /// A `fmt::Result`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Bold => write!(f, "bold"),
            Self::Fixed(weight) => write!(f, "{weight}"),
        }
    }
}

/// Trims whitespace from a string.
///
/// # Arguments
///
/// * `s` - The string to trim.
///
/// # Returns
///
/// A `Result` containing the trimmed string or an error message.
fn trim(s: &str) -> Result<String, String> {
    Ok(s.trim().to_string())
}

/// Retrieves the global settings.
///
/// # Returns
///
/// A reference to the global `Settings`.
fn cfg() -> &'static Settings {
    config::global::get()
}
