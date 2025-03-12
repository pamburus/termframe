// std imports
use std::{
    io::{self, stdout},
    process,
};

// third-party imports
use anyhow::Result;
use clap::{CommandFactory, Parser};
use env_logger::{self as logger};

// local imports
use config::Settings;
use parse::parse;
use render::svg::SvgRenderer;
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

    let options = render::Options {
        font: make_font_options(&settings, &opt)?,
        line_height: opt.line_height,
        precision: opt.precision,
        padding: render::Padding {
            x: opt.padding,
            y: opt.padding,
        },

        theme: Theme::default().into(),
        stroke: 0.2,
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

fn make_font_options(settings: &Settings, opt: &cli::Opt) -> Result<render::FontOptions> {
    let mut faces = Vec::new();

    let mut width: Option<f32> = None;
    let mut ascender: f32 = 0.0;
    let mut descender: f32 = 0.0;

    for font in &settings.fonts {
        if font.family == opt.font_family {
            for file in &font.files {
                let file = font::FontFile::load(file.as_str().into()).unwrap();
                let mut font = file.font().unwrap();

                if let Some(width) = &mut width {
                    *width = width.max(font.width());
                    ascender = ascender.max(font.ascender());
                    descender = descender.min(font.descender());
                } else {
                    width = Some(font.width());
                    ascender = font.ascender();
                    descender = font.descender();
                };

                faces.push(render::FontFace {
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
                    url: file.location().url().unwrap().to_string(),
                });
            }
        }
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
    })
}

const TERMSHOT_DEBUG_LOG: &str = "TERMSHOT_DEBUG_LOG";
const DEFAULT_FONT_METRICS: render::FontMetrics = render::FontMetrics {
    width: 0.3,
    ascender: 0.0,
    descender: 0.0,
};
