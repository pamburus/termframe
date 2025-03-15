// std imports
use std::{
    collections::HashMap,
    io::{self, stdout},
    process,
    rc::Rc,
};

// third-party imports
use anyhow::{Context, Result};
use base64::prelude::*;
use clap::{CommandFactory, Parser};
use env_logger::{self as logger};
use rayon::prelude::*;

// local imports
use config::{Patch, Settings};
use parse::parse;
use render::{CharSet, CharSetFn, svg::SvgRenderer};
use theme::Theme;

mod appdirs;
mod cli;
mod config;
mod font;
mod parse;
mod render;
mod theme;

fn main() {
    if let Err(err) = run() {
        eprintln!("ERROR: {:?}", err);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    #[allow(unused_variables)]
    let settings = bootstrap()?;

    let opt = cli::Opt::parse_from(wild::args());

    if opt.help {
        return Ok(cli::Opt::command().print_help()?);
    }

    if let Some(shell) = opt.shell_completions {
        let mut cmd = cli::Opt::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell, &mut cmd, name, &mut stdout());
        return Ok(());
    }

    if opt.man_page {
        let man = clap_mangen::Man::new(cli::Opt::command());
        man.render(&mut stdout())?;
        return Ok(());
    }

    let settings = opt.patch(settings);

    let file = std::fs::File::open(&opt.file)?;
    let input = io::BufReader::new(file);
    let surface = parse(opt.width, opt.height, input);

    let content = surface.screen_chars_to_string();

    let options = render::Options {
        font: make_font_options(&settings, content.chars().filter(|c| *c != '\n'))?,
        line_height: settings.line_height,
        precision: settings.precision,
        padding: settings.padding.convert(),
        faint_opacity: settings.faint_opacity,
        bold_is_bright: settings.bold_is_bright,
        theme: Theme::default().into(),
        stroke: settings.stroke,
        window: settings.window.convert(),
    };

    let mut output: Box<dyn io::Write> = if opt.output != "-" {
        Box::new(std::fs::File::create(opt.output)?)
    } else {
        Box::new(stdout())
    };

    let renderer = SvgRenderer::new(options);
    renderer.render(&surface, &mut output)?;

    Ok(())
}

// ---

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

// ---

fn make_font_options<C>(settings: &Settings, chars: C) -> Result<render::FontOptions>
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
            font::FontFile::load(file.as_str().into())
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

    if settings.embed_fonts {
        for (i, (_, file)) in files.iter().enumerate() {
            let data = file.data();
            log::debug!(
                "embedding font face #{i:02} with {len} bytes",
                len = data.len()
            );
            faces[i].url = format!(
                "data:font/{};base64,{}",
                file.format().unwrap_or("ttf"),
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
    ascender: 0.0,
    descender: 0.0,
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

impl Convert<render::Padding> for config::PaddingOption {
    fn convert(&self) -> render::Padding {
        let padding = self.resolve();
        render::Padding {
            left: padding.left,
            right: padding.right,
            top: padding.top,
            bottom: padding.bottom,
        }
    }
}

impl Convert<render::WindowBorder> for config::WindowBorder {
    fn convert(&self) -> render::WindowBorder {
        render::WindowBorder {
            color1: self.color1.clone(),
            color2: self.color2.clone(),
            width: self.width,
            radius: self.radius,
        }
    }
}

impl Convert<render::WindowHeader> for config::WindowHeader {
    fn convert(&self) -> render::WindowHeader {
        render::WindowHeader {
            height: self.height,
            color: self.color.clone(),
        }
    }
}

impl Convert<render::WindowButtons> for config::WindowButtons {
    fn convert(&self) -> render::WindowButtons {
        render::WindowButtons {
            radius: self.radius,
            spacing: self.spacing,
            close: self.close.convert(),
            minimize: self.minimize.convert(),
            maximize: self.maximize.convert(),
        }
    }
}

impl Convert<render::WindowButton> for config::WindowButton {
    fn convert(&self) -> render::WindowButton {
        render::WindowButton {
            color: self.color.clone(),
        }
    }
}

impl Convert<render::WindowShadow> for config::WindowShadow {
    fn convert(&self) -> render::WindowShadow {
        render::WindowShadow {
            enabled: self.enabled,
            color: self.color.clone(),
            x: self.x,
            y: self.y,
            blur: self.blur,
        }
    }
}

impl Convert<render::Window> for config::Window {
    fn convert(&self) -> render::Window {
        render::Window {
            enabled: self.enabled,
            margin: self.margin.convert(),
            border: self.border.convert(),
            header: self.header.convert(),
            buttons: self.buttons.convert(),
            shadow: self.shadow.convert(),
        }
    }
}
