// std imports
use std::{
    io::{self, stdout},
    process,
};

// third-party imports
use anyhow::Result;
use clap::{CommandFactory, Parser};
use env_logger::{self as logger};
use termwiz::surface::Surface;

// local imports
use config::Settings;
use font::Font;
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

    let input = io::BufReader::new(io::stdin().lock());
    let surface = parse(opt.width, opt.height, input);
    let ff = font::FontFile::load(
        "https://raw.githubusercontent.com/pamburus/fonts/refs/heads/main/JetBrainsMono/fonts/webfonts/JetBrainsMono-BoldItalic.woff2".into(),
    )
    .unwrap();

    let font = ff.font().unwrap();
    eprintln!(
        "weight={weight} italic={italic} bold={bold} width={w} gap={g} ascender={a} descender={d}",
        weight = font.weight(),
        italic = font.italic(),
        bold = font.bold(),
        w = font.width(),
        g = font.line_gap(),
        a = font.ascender(),
        d = font.descender()
    );

    let options = render::Options {
        font: render::FontOptions {
            family: opt.font_family,
            size: opt.font_size,
            metrics: render::FontMetrics {
                width: font.width(),
                ascender: font.ascender(),
                descender: font.descender(),
            },
        },
        line_height: opt.line_height,
        padding: render::Padding {
            x: opt.padding,
            y: opt.padding,
        },
        theme: Theme::default().into(),
        stroke: 0.2,
    };

    let renderer = SvgRenderer::new(options);
    renderer.render(&surface, &mut std::io::stdout())?;

    save(&surface, &font);
    Ok(())
}

fn save(surface: &Surface, font: &Font) {
    let theme = Theme::default();

    let mut buf = String::new();
    buf.push_str(concat!(
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg">"#,
        "\n"
    ));
    buf.push_str(STYLE);
    buf.push_str(&format!(
        r##"<rect width="100%" height="100%" fill="{background}"/>\n"##,
        background = theme.bg.to_hex_string()
    ));

    let padding = (0.0, 0.0);
    let font_size = 12.0;
    let cell_width = font_size * font.width();
    let line_interval = 1.0;
    let cell_height = font_size * line_interval;
    let stroke = 0.2;
    let bg_offset_y = (font_size - cell_height) / 2.0 + 2.0;

    buf.push_str(&format!(r#"<g font-size="{font_size}">"#));

    for (row, line) in surface.screen_lines().iter().enumerate() {
        for cluster in line.cluster(None) {
            let color = if cluster.attrs.reverse() {
                Some(theme.resolve_fg(cluster.attrs.foreground()))
            } else {
                theme.resolve(cluster.attrs.background())
            };

            if let Some(mut color) = color {
                color.a = 1.0;
                let color = color.to_hex_string();

                let x = padding.0 + cluster.first_cell_idx as f32 * cell_width - stroke;
                let y = padding.1 + row as f32 * cell_height - stroke;
                let width = cluster.width as f32 * cell_width + stroke * 2.0;
                let height = cell_height + stroke * 2.0;

                buf.push_str(&format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{width:.1}" height="{height:.1}" fill="{color}" />"#,
                ));
            }
        }
    }

    buf.push_str("\n");

    let width = surface.dimensions().0 as f32 * cell_width;

    for (row, line) in surface.screen_lines().iter().enumerate() {
        if line.is_whitespace() {
            continue;
        }

        let x = padding.0;
        let y = padding.1 + row as f32 * cell_height;

        buf.push_str(&format!(
            r##"<svg x="{x:.1}" y="{y:.1}" width="{width}" height="{cell_height}" overflow="hidden"><text fill="{fg}" y="{text_y}" xml:space="preserve">"##,
            fg = theme.fg.to_hex_string(),
            text_y = font_size-bg_offset_y,
        ));

        let mut last_cluster_was_blank = false;
        let mut prev_len = 0;
        let mut pos = 0;
        for cluster in line.cluster(None) {
            prev_len = buf.len();

            let n = cluster.first_cell_idx - pos;
            if n > 0 {
                buf.push_str(&format!(r#"<tspan>"#));
                buf.push_str(&" ".repeat(n));
                buf.push_str(r#"</tspan">"#);
            }

            let mut color = if cluster.attrs.reverse() {
                theme.resolve_bg(cluster.attrs.background())
            } else {
                theme.resolve_fg(cluster.attrs.foreground())
            };

            if cluster.attrs.intensity() == termwiz::cell::Intensity::Half {
                color = theme.bg.interpolate_lab(&color, 0.5);
            };
            color.a = 1.0;

            let fill = if color == theme.fg {
                "".into()
            } else {
                format!(r#" fill="{c}""#, c = color.to_hex_string())
            };

            let text = &cluster.text;
            last_cluster_was_blank = text.find(|x| x != ' ').is_none();

            let weight = match cluster.attrs.intensity() {
                termwiz::cell::Intensity::Bold => r#" font-weight="bold""#,
                _ => "",
            };

            let style = if cluster.attrs.italic() {
                r#" font-style="italic""#
            } else {
                ""
            };

            let decoration = if cluster.attrs.underline() != termwiz::cell::Underline::None {
                r#" text-decoration="underline""#
            } else if cluster.attrs.strikethrough() {
                r#" text-decoration="line-through""#
            } else {
                ""
            };

            let decoration = decoration.to_owned()
                + &if cluster.attrs.underline_color() != termwiz::color::ColorAttribute::Default {
                    if let Some(mut color) = theme.resolve(cluster.attrs.underline_color()) {
                        color.a = 1.0;
                        format!(r#" text-decoration-color="{c}""#, c = color.to_hex_string())
                    } else {
                        "".into()
                    }
                } else {
                    "".into()
                };

            let decoration = decoration
                + &if cluster.attrs.underline() != termwiz::cell::Underline::None {
                    format!(
                        r#" text-decoration-style="{style}""#,
                        style = match cluster.attrs.underline() {
                            termwiz::cell::Underline::None => "",
                            termwiz::cell::Underline::Single => "solid",
                            termwiz::cell::Underline::Double => "double",
                            termwiz::cell::Underline::Curly => "wavy",
                            termwiz::cell::Underline::Dotted => "dotted",
                            termwiz::cell::Underline::Dashed => "dashed",
                        }
                    )
                } else {
                    "".into()
                };

            buf.push_str(&format!(
                r#"<tspan{fill}{weight}{style}{decoration}>{text}</tspan>"#,
            ));

            pos += cluster.width;
        }
        if last_cluster_was_blank {
            buf.truncate(prev_len);
        }

        buf.push_str("</text></svg>");
    }

    buf.push_str("</g>");
    buf.push_str("</svg>");

    // Write to file.
    std::fs::write("output.svg", buf).expect("Unable to write SVG file");
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

const TERMSHOT_DEBUG_LOG: &str = "TERMSHOT_DEBUG_LOG";
const STYLE: &str = include_str!("assets/style.html");
