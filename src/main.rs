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
    buf.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#);
    buf.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    buf.push_str(r##"<rect width="100%" height="100%" fill="#282C30"/>"##);

    let cell_width = 7.225;
    let font_size = 12.0;
    let cell_height = font_size * 1.2;

    for (row, line) in surface.screen_lines().iter().enumerate() {
        for cluster in line.cluster(None) {
            let x = cluster.first_cell_idx as f32 * cell_width;
            let y = row as f32 * cell_height;
            let width = cluster.width as f32 * cell_width;

            if let Some(color) = color(cluster.attrs.background()) {
                buf.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" />"#,
                    x, y, width, cell_height, color,
                ));
            }

            // And then the text:
            buf.push_str(&format!(
                r#"<text x="{}" y="{}" fill="{}" font-family="monospace" font-size="{}">{}</text>"#,
                x,
                y + cell_height * 0.8, // Adjust vertical alignment as needed.
                color(cluster.attrs.foreground()).unwrap_or("white".into()),
                font_size,
                &cluster.text,
            ));
        }
    }
    buf.push_str("</svg>");

    // Write to file.
    std::fs::write("output.svg", buf).expect("Unable to write SVG file");
}

fn color(attr: ColorAttribute) -> Option<String> {
    match attr {
        ColorAttribute::Default => None,
        ColorAttribute::PaletteIndex(idx) => Some(
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
        ),
        ColorAttribute::TrueColorWithDefaultFallback(c)
        | ColorAttribute::TrueColorWithPaletteFallback(c, _) => Some(format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            (c.0 * 255.0) as u8,
            (c.1 * 255.0) as u8,
            (c.2 * 255.0) as u8,
            (c.3 * 255.0) as u8,
        )),
    }
}
