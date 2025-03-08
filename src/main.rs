use csscolorparser::Color;
use std::{
    io::{self, BufRead},
    str::FromStr,
};
use termwiz::{
    cell::AttributeChange,
    color::ColorAttribute,
    escape::{Action, CSI, ControlCode, csi::Sgr, parser::Parser},
    surface::{Change, SEQ_ZERO, Surface},
};

fn main() {
    let mut surface = Surface::new(160, 60);

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

    save(&surface);
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

fn save(surface: &Surface) {
    let background = "#282C30";
    let bg = Color::from_str(background).unwrap();
    let fg = Color::from_str("#acb2be").unwrap();

    let mut buf = String::new();
    buf.push_str(concat!(
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg">"#,
        "\n"
    ));
    buf.push_str(STYLE);
    buf.push_str(&format!(
        r##"<rect width="100%" height="100%" fill="{background}"/>\n"##
    ));

    let padding = (12.0, 14.0);
    let font_size = 12.0;
    let cell_width = font_size * 0.6;
    let line_interval = 1.2;
    let cell_height = font_size * line_interval;
    let stroke = 0.25;
    let bg_offset_y = (font_size - cell_height) / 2.0 + 2.0;

    buf.push_str(&format!(r#"<g font-size="{font_size}">"#));

    for (row, line) in surface.screen_lines().iter().enumerate() {
        for cluster in line.cluster(None) {
            let color = if cluster.attrs.reverse() {
                Some(to_color(cluster.attrs.foreground()).unwrap_or(fg.clone()))
            } else {
                to_color(cluster.attrs.background())
            };

            if let Some(mut color) = color {
                color.a = 1.0;
                let color = color.to_hex_string();

                let x = padding.0 + cluster.first_cell_idx as f64 * cell_width - stroke;
                let y = padding.1 + row as f64 * cell_height - stroke + bg_offset_y;
                let width = cluster.width as f64 * cell_width + stroke * 2.0;
                let height = cell_height + stroke * 2.0;

                buf.push_str(&format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{width:.1}" height="{height:.1}" fill="{color}" />"#,
                ));
            }
        }
    }

    buf.push_str("\n");

    buf.push_str(&format!(
        r##"<text x="{x:.1}" y="{y:.1}" fill="#acb2be" xml:space="preserve">"##,
        x = padding.0,
        y = padding.1 + font_size,
    ));

    let nl = &format!(r#" x="{x:.1}" dy="{line_interval:.1}em""#, x = padding.0);
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

            let mut color = if cluster.attrs.reverse() {
                to_color(cluster.attrs.background()).unwrap_or(bg.clone())
            } else {
                to_color(cluster.attrs.foreground()).unwrap_or(fg.clone())
            };

            if cluster.attrs.intensity() == termwiz::cell::Intensity::Half {
                color = bg.interpolate_lab(&color, 0.5);
            };
            color.a = 1.0;

            let fill = if color == fg {
                "".into()
            } else {
                format!(r#" fill="{c}""#, c = color.to_hex_string())
            };

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
                    if let Some(mut color) = to_color(cluster.attrs.underline_color()) {
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
                r#"<tspan{offset}{fill}{weight}{style}{decoration}>{text}</tspan>"#,
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

fn to_color(attr: ColorAttribute) -> Option<Color> {
    match attr {
        ColorAttribute::Default => None,
        ColorAttribute::PaletteIndex(idx) => Some(
            match idx {
                0 => "#282c34",  // black
                1 => "#d17277",  // red
                2 => "#a1c281",  // green
                3 => "#de9b64",  // yellow
                4 => "#74ade9",  // blue
                5 => "#bb7cd7",  // magenta
                6 => "#29a9bc",  // cyan
                7 => "#acb2be",  // white
                8 => "#676f82",  // bright black
                9 => "#e6676d",  // bright red
                10 => "#a9d47f", // bright green
                11 => "#de9b64", // bright yellow
                12 => "#66acff", // bright blue
                13 => "#c671eb", // bright magenta
                14 => "#69c6d1", // bright cyan
                15 => "#cccccc", // bright white
                _ => "#808080",
            }
            .try_into()
            .unwrap(),
        ),
        ColorAttribute::TrueColorWithDefaultFallback(c)
        | ColorAttribute::TrueColorWithPaletteFallback(c, _) => {
            Some(Color::new(c.0.into(), c.1.into(), c.2.into(), c.3.into()))
        }
    }
}

const STYLE: &str = include_str!("assets/style.html");
