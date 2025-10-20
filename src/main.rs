// std imports
use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, IsTerminal, stdout},
    process,
    rc::Rc,
};

// third-party imports
use anyhow::Context;
use base64::prelude::*;
use clap::{CommandFactory, Parser};
use csscolorparser::Color;
use enumset_ext::EnumSetExt;
use env_logger::{self as logger};
use itertools::Itertools;
use portable_pty::CommandBuilder;
use rayon::prelude::*;

// local imports
use config::{
    Load, Patch, Settings, app_dirs, load::ItemInfo, theme::ThemeConfig,
    winstyle::WindowStyleConfig,
};
use error::{AppInfoProvider, Result, UsageRequest, UsageResponse};
use font::FontFile;
use fontformat::FontFormat;
use render::{CharSet, CharSetFn, svg::SvgRenderer};
use term::Terminal;
use termwiz::color::SrgbaTuple;
use theme::{AdaptiveTheme, Theme};

// private modules
mod appdirs;
mod cli;
mod config;
mod error;
mod font;
mod fontformat;
mod help;
mod render;
mod term;
mod theme;
mod ureqmw;
mod xerr;

/// Entry point of the application
fn main() {
    let app = App::new();

    if let Err(err) = app.run() {
        err.log(&AppInfo);
        process::exit(1);
    }
}

/// Provides application-specific information
struct AppInfo;

impl AppInfoProvider for AppInfo {
    /// Suggests usage information based on the request
    fn usage_suggestion(&self, request: UsageRequest) -> Option<UsageResponse> {
        match request {
            UsageRequest::ListThemes => Some(("--list-themes".into(), "".into())),
            UsageRequest::ListWindowStyles => Some(("--list-window-styles".into(), "".into())),
        }
    }
}

/// Represents the application
struct App {
    ua: Option<ureq::Agent>,
}

impl App {
    /// Creates a new instance of the application
    fn new() -> Self {
        let mut ua = None;
        if let Some(dirs) = app_dirs() {
            ua = Some(
                ureq::Agent::config_builder()
                    .middleware(ureqmw::cache::new(&dirs.cache_dir))
                    .build()
                    .into(),
            );
        }

        Self { ua }
    }

    /// Runs the application
    fn run(&self) -> Result<()> {
        let settings = bootstrap()?;

        let opt = cli::Opt::parse_from(wild::args());

        if opt.help {
            return Ok(cli::Opt::command().print_help()?);
        }
        if let Some(shell) = opt.shell_completions {
            print_shell_completions(shell);
            return Ok(());
        }
        if opt.man_page {
            return print_man_page();
        }
        if let Some(tags) = opt.list_themes {
            return list_themes(tags);
        }
        if opt.list_window_styles {
            return list_window_styles();
        }
        if opt.list_fonts {
            return list_fonts(&settings);
        }

        let settings = Rc::new(opt.patch(settings));

        let mode = settings.mode.into();

        let theme = settings.theme.resolve(mode);
        let theme = if theme == "-" {
            AdaptiveTheme::default().resolve(mode)
        } else {
            let cfg = ThemeConfig::load_hybrid(theme)?;
            Rc::new(Theme::from_config(cfg.theme.resolve(mode)))
        };
        let window = WindowStyleConfig::load_hybrid(&settings.window.style)?.window;

        let mut terminal = Terminal::new(term::Options {
            cols: Some(
                settings
                    .terminal
                    .width
                    .initial_or(opt.width.min().or_else(|| opt.width.max()).unwrap_or(240)),
            ),
            rows: Some(
                settings.terminal.height.initial_or(
                    opt.height
                        .min()
                        .or_else(|| opt.height.max())
                        .unwrap_or(1024),
                ),
            ),
            background: Some(theme.bg.convert()),
            foreground: Some(theme.fg.convert()),
            env: settings.env.clone(),
        });

        let timeout = Some(std::time::Duration::from_secs(opt.timeout));

        if let Some(command) = &opt.command {
            let mut command = CommandBuilder::new(command);
            command.args(&opt.args);
            terminal.run(command, timeout)?;
        } else {
            if io::stdin().is_terminal() {
                return Ok(cli::Opt::command().print_help()?);
            }

            terminal.feed(io::BufReader::new(io::stdin()), io::sink())?;
        }

        if !matches!(
            (opt.width.current, opt.height.current),
            (cli::Dimension::Fixed(_), cli::Dimension::Fixed(_))
        ) {
            let width = terminal.recommended_width();
            log::info!("recommended terminal width: {width}");
            let width = opt.width.fit(width);
            terminal.set_width(width);
            let height = terminal.recommended_height();
            log::info!("recommended terminal height: {height}");
            let height = opt.height.fit(height);
            terminal.set_height(height);
            log::info!("resized terminal to {width}x{height}");
        }

        let content = terminal.surface().screen_chars_to_string();

        let options = render::Options {
            settings: settings.clone(),
            font: self.make_font_options(&settings, content.chars().filter(|c| *c != '\n'))?,
            theme,
            window,
            title: opt
                .title
                .or_else(|| command_to_title(opt.command, &opt.args)),
            mode,
            background: Some(terminal.background().convert()),
            foreground: Some(terminal.foreground().convert()),
        };

        let mut output: Box<dyn io::Write> = if opt.output != "-" {
            Box::new(std::fs::File::create(opt.output)?)
        } else {
            Box::new(stdout())
        };

        let renderer = SvgRenderer::new(options);
        renderer.render(terminal.surface(), &mut output)?;

        Ok(())
    }

    /// Creates font options based on the settings and characters
    fn make_font_options<C>(&self, settings: &Settings, chars: C) -> Result<render::FontOptions>
    where
        C: IntoIterator<Item = char>,
    {
        let mut width: Option<f32> = None;
        let mut ascender: f32 = 0.0;
        let mut descender: f32 = 0.0;

        let families = settings.font.family.resolve();

        let mut files = settings
            .fonts
            .par_iter()
            .filter(|font| families.contains(&font.family))
            .flat_map(|font| {
                font.files
                    .par_iter()
                    .rev()
                    .map(move |file| (&font.family, file))
            })
            .map(|(family, file)| {
                self.load_font(file)
                    .with_context(|| format!("failed to load font {file}"))
                    .map(|file| (family, file))
            })
            .collect::<Result<Vec<_>, _>>()?;

        files.sort_by_key(|(family, _)| {
            families
                .iter()
                .position(|f| f == *family)
                .map(|i| -(i as i64))
        });

        let mut fonts = Vec::new();

        for (family, file) in &files {
            let font = file.font().unwrap();
            let url = file.location().url().unwrap().to_string();
            fonts.push((url, family, font));
        }

        let mut used: HashMap<char, u64> = HashMap::new();

        for ch in chars {
            if used.contains_key(&ch) {
                continue;
            }

            let mut bitmap: u64 = 0;
            for (i, (_, _, font)) in fonts.iter_mut().enumerate() {
                if font.has_char(ch) {
                    bitmap |= 1 << i;
                }
            }

            log::debug!("provided by fonts {bitmap:08x?}: char {ch:<2} {ch:?}");
            used.insert(ch, bitmap);
        }

        let mut faces = Vec::new();
        let used = Rc::new(used);

        for (i, (url, family, font)) in fonts.iter_mut().enumerate().rev() {
            let mut metrics_match = true;
            if let Some(width) = &mut width {
                metrics_match = *width == font.width();
            } else {
                width = Some(font.width());
                ascender = font.ascender();
                descender = font.descender();
            };

            let used = used.clone();
            let chars = Rc::new(CharSetFn::new(move |ch| {
                (used.get(&ch).copied().unwrap_or(0) & (1 << i) as u64) != 0
            }));

            let face = make_font_face(family, url, font, chars, metrics_match);

            log::debug!(
                "font face #{i:02}: weight={weight:?} style={style:?} url={url:?}",
                weight = face.weight,
                style = face.style
            );

            faces.push(face);
        }

        faces.reverse();

        for (i, (_, family, font)) in fonts.iter_mut().enumerate() {
            log::debug!(
                "font face info #{i:02}: configured-family={cf:?} family={family:?} name={name:?}",
                family = font.family(),
                name = font.name(),
                cf = family,
            );
        }

        let metrics = if let Some(width) = width {
            render::FontMetrics {
                width,
                ascender,
                descender,
            }
        } else {
            DEFAULT_FONT_METRICS
        };

        log::debug!(
            "font metrics: width={width} ascender={ascender} descender={descender}",
            width = metrics.width,
            ascender = metrics.ascender,
            descender = metrics.descender
        );

        log::debug!(
            "prepare font faces: embed-fonts={e} subset-fonts={s}",
            e = settings.rendering.svg.embed_fonts,
            s = settings.rendering.svg.subset_fonts,
        );
        if settings.rendering.svg.embed_fonts {
            for (i, (_, file)) in files.iter().enumerate() {
                let data = if settings.rendering.svg.subset_fonts {
                    let chars = used.iter().filter(|x| *x.1 & (1 << i) != 0).map(|x| *x.0);
                    let data = fonts[i].2.subset(chars)?;
                    faces[i].format = Some(FontFormat::Ttf);
                    Cow::Owned(data)
                } else {
                    Cow::Borrowed(file.data())
                };
                log::debug!(
                    "prepare font face #{i:02} to be embedded: {len} bytes",
                    len = data.len()
                );
                faces[i].url = format!(
                    "data:{};base64,{}",
                    file.format().unwrap_or(FontFormat::Ttf).mime(),
                    BASE64_STANDARD.encode(data)
                );
            }
        }

        Ok(render::FontOptions {
            family: families,
            size: settings.font.size.into(),
            metrics,
            faces,
            weights: settings.font.weights.convert(),
        })
    }

    /// Loads a font file from a given path or URL
    fn load_font<S: AsRef<str>>(&self, file: S) -> Result<FontFile> {
        let file = file.as_ref();
        let location = font::Location::from(file);

        match location {
            font::Location::File(path) => Ok(FontFile::load_file(path)?),
            font::Location::Url(url) => {
                if let Some(ua) = &self.ua {
                    Ok(FontFile::load_url_with_agent(url, ua)?)
                } else {
                    Ok(FontFile::load_url(url)?)
                }
            }
        }
    }
}

/// Prints the manual page
fn print_man_page() -> Result<()> {
    let man = clap_mangen::Man::new(cli::Opt::command());
    man.render(&mut stdout())?;
    Ok(())
}

/// Prints shell completions for the specified shell
fn print_shell_completions(shell: clap_complete::Shell) {
    let mut cmd = cli::Opt::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut stdout());
}

/// Lists available window styles
fn list_window_styles() -> Result<()> {
    list_assets(WindowStyleConfig::list()?)
}

/// Lists available fonts
fn list_fonts(settings: &Settings) -> Result<()> {
    for font in &settings.fonts {
        println!("{}", font.family);
    }
    Ok(())
}

/// Lists available themes based on the provided tags
fn list_themes(tags: Option<cli::ThemeTagSet>) -> Result<()> {
    let items = ThemeConfig::list()?;
    let mut formatter = help::Formatter::new(stdout());

    formatter.format_grouped_list(
        items
            .into_iter()
            .filter(|(name, _)| {
                if let Some(tags) = tags {
                    ThemeConfig::load(name)
                        .ok()
                        .map(|theme| theme.tags.includes(*tags))
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .sorted_by_key(|x| (x.1.origin, x.0.clone()))
            .chunk_by(|x| x.1.origin)
            .into_iter()
            .map(|(origin, group)| (origin, group.map(|x| x.0))),
    )?;
    Ok(())
}

/// Lists assets based on the provided items
fn list_assets(items: impl IntoIterator<Item = (String, ItemInfo)>) -> Result<()> {
    let mut formatter = help::Formatter::new(stdout());

    formatter.format_grouped_list(
        items
            .into_iter()
            .sorted_by_key(|x| (x.1.origin, x.0.clone()))
            .chunk_by(|x| x.1.origin)
            .into_iter()
            .map(|(origin, group)| (origin, group.map(|x| x.0))),
    )?;
    Ok(())
}

/// Bootstraps the application settings
fn bootstrap() -> Result<Settings> {
    if std::env::var(TERMFRAME_DEBUG_LOG).is_ok() {
        logger::Builder::from_env(TERMFRAME_DEBUG_LOG)
            .format_timestamp_micros()
            .init();
        log::debug!("logging initialized");
    } else {
        logger::Builder::new()
            .filter_level(log::LevelFilter::Warn)
            .format_timestamp_millis()
            .init()
    }

    let opt = cli::BootstrapOpt::parse().args;

    let (offset, no_default_configs) = opt
        .config
        .iter()
        .rposition(|x| x.is_empty() || x == "-")
        .map(|x| (x + 1, true))
        .unwrap_or_default();
    let configs = &opt.config[offset..];

    let settings = config::at(configs).no_default(no_default_configs).load()?;
    config::global::initialize(settings.clone());

    Ok(settings)
}

/// Creates a font face based on the provided parameters
fn make_font_face(
    family: &str,
    url: &mut String,
    font: &mut font::Font,
    chars: Rc<dyn CharSet>,
    metrics_match: bool,
) -> render::FontFace {
    if let Some(ff) = font.family()
        && ff != family
    {
        log::warn!("font family mismatch for {url}: expected {family:?}, got {ff:?}",);
    }

    render::FontFace {
        family: family.to_owned(),
        weight: if let Some((min, max)) = font.weight_axis() {
            render::FontWeight::Variable(f32::from(min) as u16, f32::from(max) as u16)
        } else if font.bold() {
            render::FontWeight::Bold
        } else if font.weight() == 400 {
            render::FontWeight::Normal
        } else {
            render::FontWeight::Fixed(font.weight())
        },
        style: if font.italic() {
            Some(render::FontStyle::Italic)
        } else if font.has_italic_axis() {
            None
        } else {
            Some(render::FontStyle::Normal)
        },
        format: font.format(),
        url: url.clone(),
        chars,
        metrics_match,
    }
}

const TERMFRAME_DEBUG_LOG: &str = "TERMFRAME_DEBUG_LOG";
const DEFAULT_FONT_METRICS: render::FontMetrics = render::FontMetrics {
    width: 0.6,
    ascender: 1.02,
    descender: -0.3,
};

/// Trait for converting between types
trait Convert<T> {
    fn convert(&self) -> T;
}

impl Convert<render::FontWeight> for config::FontWeight {
    fn convert(&self) -> render::FontWeight {
        match self {
            config::FontWeight::Normal => render::FontWeight::Normal,
            config::FontWeight::Bold => render::FontWeight::Bold,
            config::FontWeight::Fixed(weight) => render::FontWeight::Fixed(*weight),
        }
    }
}

impl Convert<render::FontWeights> for config::FontWeights {
    fn convert(&self) -> render::FontWeights {
        render::FontWeights {
            normal: self.normal.convert(),
            bold: self.bold.convert(),
            faint: self.faint.convert(),
        }
    }
}

impl Convert<SrgbaTuple> for Color {
    fn convert(&self) -> SrgbaTuple {
        let x = self.to_rgba8();
        (x[0], x[1], x[2], x[3]).into()
    }
}

impl Convert<Color> for SrgbaTuple {
    fn convert(&self) -> Color {
        self.as_rgba_u8().into()
    }
}

/// Converts a command and its arguments into a title string
fn command_to_title(
    command: Option<impl AsRef<str>>,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Option<String> {
    use shell_escape::escape;

    Some(
        std::iter::once(escape(command?.as_ref().into()))
            .chain(
                args.into_iter()
                    .map(|arg| escape(arg.as_ref().to_owned().into())),
            )
            .join(" "),
    )
}
