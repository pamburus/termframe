// third-party imports
use clap::{ArgAction, Args, Parser, value_parser};
use clap_complete::Shell;

// local imports
use crate::config;

// ---

/// JSON and logfmt log converter to human readable representation.
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

    /// Padding for the inner text.
    #[arg(long, default_value_t = config::global::get().padding, overrides_with = "padding")]
    pub padding: f32,

    /// Font family.
    #[arg(long, default_value = &config::global::get().font.family, overrides_with = "font_family")]
    pub font_family: String,

    /// Font size.
    #[arg(long, default_value_t = config::global::get().font.size, overrides_with = "font_size")]
    pub font_size: f32,

    /// Font file, can be specified multiple times.
    #[arg(long, num_args = 1)]
    pub font_file: Vec<String>,

    #[arg(long, default_value_t = config::global::get().font.weights.normal.into(), overrides_with = "font_weight")]
    pub font_weight: FontWeight,

    #[arg(long, default_value_t = config::global::get().font.weights.bold.into(), overrides_with = "font_weight_bold")]
    pub font_weight_bold: FontWeight,

    #[arg(long, default_value_t = config::global::get().font.weights.faint.into(), overrides_with = "font_weight_faint")]
    pub font_weight_faint: FontWeight,

    #[arg(long, default_value_t = config::global::get().faint_opacity, overrides_with = "faint_opacity")]
    pub faint_opacity: f32,

    /// Line height.
    #[arg(long, default_value_t = config::global::get().line_height, overrides_with = "line_height")]
    pub line_height: f32,

    /// Precision for floating point numbers.
    #[arg(long, default_value_t = config::global::get().precision, overrides_with = "precision")]
    pub precision: u8,

    /// Theme.
    #[arg(long, default_value = &config::global::get().theme, overrides_with = "theme")]
    pub theme: String,

    /// First line to capture.
    /// If not specified, captures since the beginning of the input.
    #[arg(long, overrides_with = "start")]
    pub start: Option<usize>,

    /// Last line to capture.
    /// If not specified, captures until the end of the input.
    #[arg(long, overrides_with = "end")]
    pub end: Option<usize>,

    /// Output file.
    /// If not specified, prints to stdout.
    #[arg(long, short = 'o', overrides_with = "output")]
    pub output: Option<String>,

    /// Print help.
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

// ---

#[derive(Args)]
pub struct BootstrapArgs {
    /// Configuration file path.
    #[arg(long, value_name = "FILE", env = "HL_CONFIG", num_args = 1)]
    pub config: Vec<String>,
}

/// JSON and logfmt log converter to human readable representation.
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

impl Into<crate::render::FontWeight> for FontWeight {
    fn into(self) -> crate::render::FontWeight {
        match self {
            Self::Normal => crate::render::FontWeight::Normal,
            Self::Bold => crate::render::FontWeight::Bold,
            Self::Fixed(weight) => crate::render::FontWeight::Fixed(weight),
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
