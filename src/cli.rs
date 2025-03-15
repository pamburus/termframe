// third-party imports
use clap::{ArgAction, Args, Parser, value_parser};
use clap_complete::Shell;

// local imports
use crate::config::{self, FontFamilyOption, PaddingOption, Settings};

// ---

/// Terminal output snapshot tool.
#[derive(Parser)]
#[clap(version, disable_help_flag = true)]
pub struct Opt {
    #[command(flatten)]
    pub bootstrap: BootstrapArgs,

    /// Width of the virtual terminal window.
    #[arg(long, short = 'W', default_value_t = config::global::get().terminal.width, overrides_with = "width")]
    pub width: usize,

    /// Height of the virtual terminal window.
    #[arg(long, short = 'H', default_value_t = config::global::get().terminal.height, overrides_with = "height")]
    pub height: usize,

    /// Override padding for the inner text in font size units.
    #[arg(long, overrides_with = "padding")]
    pub padding: Option<f32>,

    /// Font family, can be specified multiple times.
    #[arg(long)]
    pub font_family: Vec<String>,

    /// Font size.
    #[arg(long, default_value_t = config::global::get().font.size, overrides_with = "font_size")]
    pub font_size: f32,

    /// Normal font weight.
    #[arg(long, default_value_t = config::global::get().font.weights.normal.into(), overrides_with = "font_weight")]
    pub font_weight: FontWeight,

    /// Embed fonts.
    #[arg(long, num_args = 1, default_value_t = config::global::get().embed_fonts, overrides_with = "embed_fonts")]
    pub embed_fonts: bool,

    /// Use bright colors for bold text.
    #[arg(long, num_args = 1, default_value_t = config::global::get().bold_is_bright, overrides_with = "bold_is_bright")]
    pub bold_is_bright: bool,

    /// Bold text font weight.
    #[arg(long, default_value_t = config::global::get().font.weights.bold.into(), overrides_with = "bold_font_weight")]
    pub bold_font_weight: FontWeight,

    /// Faint text opacity.
    #[arg(long, default_value_t = config::global::get().faint_opacity, overrides_with = "faint_opacity")]
    pub faint_opacity: f32,

    // Faint text font weight.
    #[arg(long, default_value_t = config::global::get().font.weights.faint.into(), overrides_with = "faint_font_weight")]
    pub faint_font_weight: FontWeight,

    /// Line height.
    #[arg(long, default_value_t = config::global::get().line_height, overrides_with = "line_height")]
    pub line_height: f32,

    /// Precision for floating point numbers.
    #[arg(long, default_value_t = config::global::get().precision, overrides_with = "precision")]
    pub precision: u8,

    /// Theme.
    #[arg(long, default_value = &config::global::get().theme, overrides_with = "theme")]
    pub theme: String,

    /// Enable window.
    #[arg(long, num_args = 1, default_value_t = config::global::get().window.enabled, overrides_with = "window")]
    pub window: bool,

    /// Enable window shadow.
    #[arg(long, num_args = 1, default_value_t = config::global::get().window.shadow.enabled, overrides_with = "window_shadow")]
    pub window_shadow: bool,

    /// Override window margin, in pixels.
    #[arg(long, overrides_with = "window_margin")]
    pub window_margin: Option<f32>,

    /// First line to capture, if not specified, captures from the beginning of the input.
    #[arg(long, overrides_with = "start")]
    pub start: Option<usize>,

    /// Last line to capture, if not specified, captures to the end of the input.
    #[arg(long, overrides_with = "end")]
    pub end: Option<usize>,

    /// Output file, by default prints to stdout.
    #[arg(long, short = 'o', default_value = "-", overrides_with = "output")]
    pub output: String,

    /// Print help and exit.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub help: bool,

    /// Print shell auto-completion script and exit.
    #[arg(
        long,
        value_parser = value_parser!(Shell),
        value_name = "SHELL",
    )]
    pub shell_completions: Option<Shell>,

    /// Print man page and exit.
    #[arg(long)]
    pub man_page: bool,

    /// File to process
    #[arg(name = "FILE")]
    pub file: std::path::PathBuf,
}

impl config::Patch for Opt {
    fn patch(&self, settings: Settings) -> Settings {
        let mut settings = settings;

        settings.terminal.width = self.width;
        settings.terminal.height = self.height;
        if self.font_family.len() != 0 {
            settings.font.family = FontFamilyOption::Multiple(self.font_family.clone());
        }
        settings.font.size = self.font_size;
        settings.font.weights.normal = self.font_weight.into();
        settings.font.weights.bold = self.bold_font_weight.into();
        settings.font.weights.faint = self.faint_font_weight.into();
        settings.embed_fonts = self.embed_fonts;
        settings.faint_opacity = self.faint_opacity;
        settings.line_height = self.line_height;
        settings.precision = self.precision;
        settings.bold_is_bright = self.bold_is_bright;
        settings.theme = self.theme.clone();
        if let Some(padding) = self.padding {
            settings.padding = PaddingOption::Uniform(padding);
        }
        settings.window.enabled = self.window;
        settings.window.shadow.enabled = self.window_shadow;
        if let Some(margin) = self.window_margin {
            settings.window.margin = PaddingOption::Uniform(margin);
        }

        settings
    }
}

// ---

#[derive(Args)]
pub struct BootstrapArgs {
    /// Configuration file path.
    #[arg(long, value_name = "FILE", env = "TERMSHOT_CONFIG", num_args = 1)]
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
    pub fn parse() -> Self {
        Self::parse_from(Self::args())
    }

    pub fn args() -> Vec<String> {
        let mut args = wild::args();
        let Some(first) = args.next() else {
            return vec![];
        };

        let mut result = vec![first];
        let mut follow_up = false;

        while let Some(arg) = args.next() {
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

// ---

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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            s => match s.parse() {
                Ok(weight) => Ok(Self::Fixed(weight)),
                Err(_) => Err(format!("Invalid font weight: {}", s)),
            },
        }
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
