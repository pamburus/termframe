// std imports
use std::{
    collections::HashMap,
    io::{self, stdout},
    process,
    rc::Rc,
};

// third-party imports
use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use env_logger::{self as logger};
use rayon::prelude::*;

// local imports
use config::Settings;
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

    let file = std::fs::File::open(&opt.file)?;
    let input = io::BufReader::new(file);
    let surface = parse(opt.width, opt.height, input);

    let content = surface.screen_chars_to_string();

    let options = render::Options {
        font: make_font_options(&settings, &opt, content.chars().filter(|c| *c != '\n'))?,
        line_height: opt.line_height,
        precision: opt.precision,
        padding: render::Padding {
            x: opt.padding,
            y: opt.padding,
        },
        faint_opacity: opt.faint_opacity,
        theme: Theme::default().into(),
        stroke: opt.stroke,
    };

    let mut output: Box<dyn io::Write> = if let Some(output) = opt.output {
        Box::new(std::fs::File::create(output)?)
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

fn make_font_options<C>(
    settings: &Settings,
    opt: &cli::Opt,
    chars: C,
) -> Result<render::FontOptions>
where
    C: IntoIterator<Item = char>,
{
    let mut faces = Vec::new();

    let mut width: Option<f32> = None;
    let mut ascender: f32 = 0.0;
    let mut descender: f32 = 0.0;

    let files = settings
        .fonts
        .par_iter()
        .filter(|font| font.family == opt.font_family)
        .flat_map(|font| &font.files)
        .map(|file| {
            font::FontFile::load(file.as_str().into())
                .with_context(|| format!("failed to load font {file}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut fonts = Vec::new();

    for file in &files {
        let font = file.font().unwrap();
        let url = file.location().url().unwrap().to_string();
        fonts.push((url, font));
    }

    let mut used: HashMap<char, u64> = HashMap::new();

    for ch in chars {
        let mut bitmap: u64 = 0;
        for (i, (_, font)) in fonts.iter_mut().enumerate() {
            if font.has_char(ch) {
                bitmap |= 1 << i;
            }
        }
        used.insert(ch, bitmap);
    }

    let used = Rc::new(used);

    for (i, (url, font)) in fonts.iter_mut().enumerate() {
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

        let face = make_font_face(url, font, chars);

        log::debug!(
            "font face #{i}: weight={weight:?} style={style:?} url={url:?}",
            weight = face.weight,
            style = face.style
        );

        faces.push(face);
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

    Ok(render::FontOptions {
        family: opt.font_family.clone(),
        size: opt.font_size,
        metrics,
        faces,
        weights: render::FontWeights {
            normal: opt.font_weight.into(),
            bold: opt.font_weight_bold.into(),
            faint: opt.font_weight_faint.into(),
        },
    })
}

fn make_font_face(
    url: &mut String,
    font: &mut font::Font,
    chars: Rc<dyn CharSet>,
) -> render::FontFace {
    render::FontFace {
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
