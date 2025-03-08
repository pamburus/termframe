use std::io::{self, BufRead};
use termwiz::{
    cell::AttributeChange,
    color::ColorAttribute,
    escape::{Action, CSI, ControlCode, csi::Sgr, parser::Parser},
    surface::{Change, SEQ_ZERO, Surface},
};

fn main() {
    let mut surface = Surface::new(120, 60);

    let mut parser = Parser::new();
    let mut reader = io::BufReader::new(io::stdin().lock());

    while let Ok(buffer) = reader.fill_buf() {
        if buffer.is_empty() {
            break;
        }

        parser.parse(buffer, |action| {
            apply_action_to_surface(&mut surface, action);
        });

        let len = buffer.len();
        reader.consume(len);
    }

    save(&mut surface);
}

// A function to convert an Action into a vector of Changes.
fn apply_action_to_surface(surface: &mut Surface, action: Action) {
    match action {
        Action::Print(ch) => surface.add_change(ch),
        Action::PrintString(s) => surface.add_change(s),
        Action::Control(code) => match code {
            ControlCode::LineFeed => surface.add_change("\r\n"),
            ControlCode::CarriageReturn | ControlCode::HorizontalTab => {
                surface.add_change(code as u8 as char)
            }
            _ => SEQ_ZERO,
        },
        Action::CSI(csi) => {
            match csi {
                CSI::Sgr(sgr) => match sgr {
                    Sgr::Reset => surface.add_change(Change::AllAttributes(Default::default())),
                    Sgr::Intensity(intensity) => {
                        surface.add_change(Change::Attribute(AttributeChange::Intensity(intensity)))
                    }
                    Sgr::Underline(underline) => {
                        surface.add_change(Change::Attribute(AttributeChange::Underline(underline)))
                    }
                    Sgr::UnderlineColor(_) => SEQ_ZERO,
                    Sgr::Blink(_) => SEQ_ZERO,
                    Sgr::Inverse(inverse) => {
                        surface.add_change(Change::Attribute(AttributeChange::Reverse(inverse)))
                    }
                    Sgr::Foreground(color) => surface
                        .add_change(Change::Attribute(AttributeChange::Foreground(color.into()))),
                    Sgr::Background(color) => surface
                        .add_change(Change::Attribute(AttributeChange::Background(color.into()))),
                    Sgr::Italic(italic) => {
                        surface.add_change(Change::Attribute(AttributeChange::Italic(italic)))
                    }
                    Sgr::StrikeThrough(enabled) => surface
                        .add_change(Change::Attribute(AttributeChange::StrikeThrough(enabled))),
                    Sgr::Invisible(enabled) => {
                        surface.add_change(Change::Attribute(AttributeChange::Invisible(enabled)))
                    }
                    Sgr::Font(_) => SEQ_ZERO,
                    Sgr::VerticalAlign(_) => SEQ_ZERO,
                    Sgr::Overline(_) => SEQ_ZERO,
                },
                _ => SEQ_ZERO,
            }
        }
        _ => SEQ_ZERO,
    };
}

fn save(surface: &mut Surface) {
    let mut buf = String::new();
    buf.push_str(concat!(
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg">"#,
        "\n"
    ));
    buf.push_str(STYLE);
    buf.push_str(concat!(
        r##"<rect width="100%" height="100%" fill="#282C30"/>"##,
        "\n"
    ));

    let margin_x = 12.0;
    let margin_y = 14.0;
    let font_size = 12.0;
    let cell_width = 7.2;
    let line_interval = 1.2;
    let cell_height = font_size * line_interval;
    let extra = 0.25;

    buf.push_str(&format!(r#"<g font-size="{font_size}">"#));

    for (row, line) in surface.screen_lines().iter().enumerate() {
        for cluster in line.cluster(None) {
            if let Some((color, opacity)) = color(cluster.attrs.background()) {
                let x = margin_x + cluster.first_cell_idx as f64 * cell_width - extra;
                let y = margin_y + row as f64 * cell_height - extra;
                let width = cluster.width as f64 * cell_width + extra * 2.0;
                let height = cell_height + extra * 2.0;

                let opacity = if opacity < 1.0 {
                    format!(r#" opacity="{:.1}""#, opacity)
                } else {
                    "".into()
                };

                buf.push_str(&format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{width:.1}" height="{height:.1}" fill="{color}"{opacity} />"#,
                ));
            }
        }
    }

    buf.push_str("\n");

    let (mx, my) = (margin_x, margin_y as f64 + font_size);
    buf.push_str(&format!(
        r##"<text x="{mx}" y="{my:.0}" fill="#acb2be" xml:space="preserve">"##,
    ));

    let nl = &format!(r#" x="{mx}" dy="{line_interval:.1}em""#);
    let mut offset = "";
    for line in surface.screen_lines().iter() {
        let mut pos = 0;
        for cluster in line.cluster(None) {
            let n = cluster.first_cell_idx - pos;
            if n > 0 {
                buf.push_str(&format!(r#"<tspan {}>"#, offset));
                buf.push_str(&" ".repeat(n));
                buf.push_str(r#"</tspan">"#);
                offset = "";
            }

            let (fill, opacity) = color(cluster.attrs.foreground())
                .map(|(c, o)| {
                    (
                        format!(r#" fill="{c}""#),
                        if o < 1.0 {
                            format!(r#" opacity="{c}""#)
                        } else {
                            "".into()
                        },
                    )
                })
                .unwrap_or_default();

            let text = &cluster.text;

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
                    let (c, _) = color(cluster.attrs.underline_color()).unwrap();
                    format!(r#" text-decoration-color="{c}""#, c = c)
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
                r#"<tspan{offset}{fill}{opacity}{weight}{style}{decoration}>{text}</tspan>"#,
            ));
            offset = "";

            pos += cluster.width;
        }
        offset = nl;
    }

    buf.push_str("</text>");
    buf.push_str("</g>");
    buf.push_str("</svg>");

    // Write to file.
    std::fs::write("output.svg", buf).expect("Unable to write SVG file");
}

fn color(attr: ColorAttribute) -> Option<(String, f32)> {
    match attr {
        ColorAttribute::Default => None,
        ColorAttribute::PaletteIndex(idx) => Some((
            match idx {
                0 => "black",
                1 => "red",
                2 => "green",
                3 => "yellow",
                4 => "blue",
                5 => "magenta",
                6 => "cyan",
                7 => "white",
                8 => "bright-black",
                9 => "bright-red",
                10 => "bright-green",
                11 => "bright-yellow",
                12 => "bright-blue",
                13 => "bright-magenta",
                14 => "bright-cyan",
                15 => "bright-white",
                _ => "white",
            }
            .into(),
            1.0,
        )),
        ColorAttribute::TrueColorWithDefaultFallback(c)
        | ColorAttribute::TrueColorWithPaletteFallback(c, _) => Some((
            format!(
                "#{:02x}{:02x}{:02x}",
                (c.0 * 255.0) as u8,
                (c.1 * 255.0) as u8,
                (c.2 * 255.0) as u8,
            ),
            c.3,
        )),
    }
}

const STYLE: &str = include_str!("assets/style.html");
