// std imports
use std::{
    collections::HashMap,
    io::{self, IsTerminal, stdout},
    process,
    rc::Rc,
    time::Duration,
};

// third-party imports
use anyhow::Context;
use base64::prelude::*;
use cached::{
    IOCached,
    stores::{DiskCache, DiskCacheBuilder},
};
use clap::{CommandFactory, Parser};
use csscolorparser::Color;
use env_logger::{self as logger};
use itertools::Itertools;
use nu_ansi_term::Color as NuColor;
use portable_pty::CommandBuilder;
use rayon::prelude::*;

// local imports
use config::{
    Load, Patch, Settings,
    load::{ItemInfo, Origin},
    theme::ThemeConfig,
    winstyle::WindowStyleConfig,
};
use error::{AppInfoProvider, Result, UsageRequest, UsageResponse};
use fontformat::FontFormat;
use render::{CharSet, CharSetFn, svg::SvgRenderer};
use term::Terminal;
use termwiz::color::SrgbaTuple;
use theme::{AdaptiveTheme, Theme};

mod appdirs;
mod cli;
mod config;
mod error;
mod font;
mod fontformat;
mod render;
mod term;
mod theme;
mod xerr;

fn main() {
    let app = App::new();

    if let Err(err) = app.run() {
        err.log(&AppInfo);
        process::exit(1);
    }
}

struct AppInfo;

impl AppInfoProvider for AppInfo {
    fn usage_suggestion(&self, request: UsageRequest) -> Option<UsageResponse> {
        match request {
            UsageRequest::ListThemes => Some(("--list-themes".into(), "".into())),
            UsageRequest::ListWindowStyles => Some(("--list-window-styles".into(), "".into())),
        }
    }
}

struct App {
    cache: Option<DiskCache<String, Vec<u8>>>,
}

impl App {
    fn new() -> Self {
        let mut cache = None;

        if let Some(dirs) = config::app_dirs() {
            cache = DiskCacheBuilder::new("main")
                .set_disk_directory(dirs.cache_dir.join(config::APP_NAME))
                .set_lifespan(3600)
                .build()
                .map_err(|e| log::warn!("Failed to create disk cache: {e}"))
                .ok();
        }

        Self { cache }
    }

    fn run(&self) -> Result<()> {
        let settings = bootstrap()?;

        let opt = cli::Opt::parse_from(wild::args());

        if opt.help {
            return Ok(cli::Opt::command().print_help()?);
        }

        if let Some(shell) = opt.shell_completions {
            return Ok(print_shell_completions(shell));
        }

        if opt.man_page {
            return Ok(print_man_page()?);
        }

        if opt.list_themes {
            return Ok(list_themes()?);
        }

        if opt.list_window_styles {
            return Ok(list_window_styles()?);
        }

        let settings = Rc::new(opt.patch(settings));

        let mode = settings.mode.into();

        let theme = settings.theme.resolve(mode);
        let theme = if theme == "-" {
            AdaptiveTheme::default().resolve(mode)
        } else {
            Rc::new(Theme::from_config(ThemeConfig::load(theme)?.resolve(mode)))
        };
        let window = WindowStyleConfig::load(&settings.window.style)?.window;

        let mut terminal = Terminal::new(term::Options {
            cols: Some(opt.width.into()),
            rows: Some(opt.height.into()),
            background: Some(theme.bg.convert()),
            foreground: Some(theme.fg.convert()),
        })?;

        if let Some(command) = opt.command {
            let mut command = CommandBuilder::new(command);
            command.args(&opt.args);
            terminal.run(command, Some(Duration::from_secs(opt.timeout)))?;
        } else {
            if io::stdin().is_terminal() {
                return Ok(cli::Opt::command().print_help()?);
            }

            terminal.feed(io::BufReader::new(io::stdin()), &mut io::sink())?;
        }

        let content = terminal.surface().screen_chars_to_string();

        let options = render::Options {
            settings: settings.clone(),
            font: self.make_font_options(&settings, content.chars().filter(|c| *c != '\n'))?,
            theme,
            window,
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
        renderer.render(&terminal.surface(), &mut output)?;

        Ok(())
    }

    fn make_font_options<C>(&self, settings: &Settings, chars: C) -> Result<render::FontOptions>
    where
        C: IntoIterator<Item = char>,
    {
        let mut faces = Vec::new();

        let mut width: Option<f32> = None;
        let mut ascender: f32 = 0.0;
        let mut descender: f32 = 0.0;

        let families = settings.font.family.resolve();

        let mut files = settings
            .fonts
            .par_iter()
            .filter(|font| families.contains(&font.family))
            .flat_map(|font| font.files.par_iter().map(move |file| (&font.family, file)))
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
            let mut bitmap: u64 = 0;
            for (i, (_, _, font)) in fonts.iter_mut().enumerate() {
                if font.has_char(ch) {
                    bitmap |= 1 << i;
                }
            }
            used.insert(ch, bitmap);
        }

        let used = Rc::new(used);

        for (i, (url, family, font)) in fonts.iter_mut().enumerate() {
            if let Some(width) = &mut width {
                *width = width.max(font.width());
                ascender = ascender.max(font.ascender());
                descender = descender.min(font.descender());
            } else {
                width = Some(font.width());
                ascender = font.ascender();
                descender = font.descender();
            };

            let used = used.clone();
            let chars = Rc::new(CharSetFn::new(move |ch| {
                (used.get(&ch).copied().unwrap_or(0) & (1 << i) as u64) != 0
            }));

            let face = make_font_face(family, url, font, chars);

            log::debug!(
                "font face #{i:02}: weight={weight:?} style={style:?} url={url:?}",
                weight = face.weight,
                style = face.style
            );

            faces.push(face);
        }

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

        if settings.embed_fonts {
            for (i, (_, file)) in files.iter().enumerate() {
                let data = file.data();
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
            size: settings.font.size,
            metrics,
            faces,
            weights: settings.font.weights.convert(),
        })
    }

    fn load_font<S: AsRef<str>>(&self, file: S) -> Result<font::FontFile> {
        let file = file.as_ref();
        let location = file.into();

        let Some(cache) = &self.cache else {
            return Ok(font::FontFile::load(location)?);
        };

        let key = format!("font:{file}");
        let file = if let Some(data) = cache.cache_get(&key)? {
            log::debug!("loading font from cache: {file}");
            font::FontFile::load_bytes(&data, location)?
        } else {
            let file = font::FontFile::load(location)?;
            cache.cache_set(key, file.data().to_owned())?;
            file
        };
        Ok(file)
    }
}

fn print_man_page() -> Result<()> {
    let man = clap_mangen::Man::new(cli::Opt::command());
    man.render(&mut stdout())?;
    Ok(())
}

fn print_shell_completions(shell: clap_complete::Shell) {
    let mut cmd = cli::Opt::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut stdout());
}

fn list_themes() -> Result<()> {
    list_assets(ThemeConfig::list()?)
}

fn list_window_styles() -> Result<()> {
    list_assets(WindowStyleConfig::list()?)
}

fn list_assets(items: HashMap<String, ItemInfo>) -> Result<()> {
    let mut items: Vec<_> = items.into_iter().collect();
    items.sort_by_key(|(name, info)| (info.origin.clone(), name.clone()));

    let term = if stdout().is_terminal() {
        term_size::dimensions()
    } else {
        None
    };

    let max_len = if term.is_some() {
        items
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or_default()
    } else {
        0
    };

    let columns = match term {
        Some((w, _)) => w / (max_len + 4),
        None => 1,
    };

    for (origin, group) in items
        .into_iter()
        .chunk_by(|(_, info)| info.origin.clone())
        .into_iter()
    {
        let origin_str = match origin {
            Origin::Stock => "stock",
            Origin::Custom => "custom",
        };

        if term.is_some() {
            println!("{}:", NuColor::Default.bold().paint(origin_str));
        }

        let group: Vec<_> = group.collect();
        let rows = (group.len() + columns - 1) / columns;

        for row in 0..rows {
            for col in 0..columns {
                if let Some((name, _)) = group.get(row + col * rows) {
                    if term.is_some() {
                        print!("• {:width$}", name, width = max_len + 2);
                    } else {
                        println!("{}", name);
                    }
                }
            }
            if term.is_some() {
                println!();
            }
        }
    }
    Ok(())
}

fn bootstrap() -> Result<Settings> {
    if std::env::var(TERMSHOT_DEBUG_LOG).is_ok() {
        logger::Builder::from_env(TERMSHOT_DEBUG_LOG)
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
        .rposition(|x| x == "" || x == "-")
        .map(|x| (x + 1, true))
        .unwrap_or_default();
    let configs = &opt.config[offset..];

    let settings = config::at(configs).no_default(no_default_configs).load()?;
    config::global::initialize(settings.clone());

    Ok(settings)
}

fn make_font_face(
    family: &str,
    url: &mut String,
    font: &mut font::Font,
    chars: Rc<dyn CharSet>,
) -> render::FontFace {
    if let Some(ff) = font.family() {
        if ff != family {
            log::warn!("font family mismatch for {url}: expected {family:?}, got {ff:?}",);
        }
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
    }
}

const TERMSHOT_DEBUG_LOG: &str = "TERMSHOT_DEBUG_LOG";
const DEFAULT_FONT_METRICS: render::FontMetrics = render::FontMetrics {
    width: 0.6,
    ascender: 1.02,
    descender: -0.3,
};

// ---

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
