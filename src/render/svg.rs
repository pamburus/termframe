// std imports
use std::ops::Range;

// third-party imports
use askama::Template;
use svg::{Document, node::element};
use termwiz::{
    cell::{Intensity, Underline},
    cellcluster::CellCluster,
    color::ColorAttribute,
    surface::{Line, Surface, line::CellRef},
};

// local imports
use super::{Padding, Render};

// re-exports
pub use super::{Options, Result};

pub struct SvgRenderer {
    options: Options,
}

impl SvgRenderer {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    pub fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()> {
        let opt = &self.options;

        const FP: u8 = 3; // floating point precision
        let dimensions = surface.dimensions(); // surface dimensions in cells
        let size = (
            // terminal surface size in pixels
            (dimensions.0 as f32 * opt.font.size * opt.font.metrics.width).r2p(FP),
            (dimensions.1 as f32 * opt.font.size * opt.line_height).r2p(FP),
        );
        let pad = opt.padding.r2p(FP); // padding in pixels
        let outer = (size.0 + pad.x * 2.0, size.1 + pad.y * 2.0).r2p(FP); // outer dimensions in pixels
        let lh = opt.line_height.r2p(FP); // line height in em
        let tyo = ((lh + opt.font.metrics.descender + opt.font.metrics.ascender) / 2.0).r2p(FP); // text y-offset in em
        let fw = opt.font.metrics.width.r2p(FP); // font width in em
        let cw = (opt.font.size * opt.font.metrics.width).r2p(FP); // column width in pixels
        let ch = (opt.font.size * opt.line_height).r2p(FP); // column height in pixels

        let background = element::Rectangle::new()
            .set("x", -pad.x)
            .set("y", -pad.y)
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", opt.theme.bg.to_hex_string());

        let style = element::Style::new(
            styles::Screen {
                font_family: &format!("{:?}", opt.font.family.as_str()),
                font_size: opt.font.size,
                fill: &opt.theme.fg.to_hex_string(),
            }
            .render()?,
        );

        let mut group = element::Group::new().set("class", "screen");

        for (row, line) in surface.screen_lines().iter().enumerate() {
            for cluster in line.cluster(None) {
                let color = if cluster.attrs.reverse() {
                    Some(opt.theme.resolve_fg(cluster.attrs.foreground()))
                } else {
                    opt.theme.resolve(cluster.attrs.background())
                };

                if let Some(mut color) = color {
                    color.a = 1.0;

                    let x = (cluster.first_cell_idx as f32 * cw - opt.stroke).r2p(FP);
                    let y = (row as f32 * ch - opt.stroke).r2p(FP);
                    let width = (cluster.width as f32 * cw + opt.stroke * 2.0).r2p(FP);
                    let height = (ch + opt.stroke * 2.0).r2p(FP);

                    let rect = element::Rectangle::new()
                        .set("x", x)
                        .set("y", y)
                        .set("width", width)
                        .set("height", height)
                        .set("fill", color.to_hex_string());

                    group = group.add(rect);
                }
            }
        }

        for (row, line) in surface.screen_lines().iter().enumerate() {
            if line.is_whitespace() {
                continue;
            }

            let mut sl = element::SVG::new()
                .set("y", format!("{}em", (row as f32 * lh).r2p(FP)))
                .set("width", size.0)
                .set("height", format!("{}em", lh))
                .set("overflow", "hidden");

            let mut tl = element::Text::new("")
                .set("y", format!("{}em", tyo))
                .set("xml:space", "preserve");

            let mut pos = 0;
            for cluster in line.cluster(None) {
                if cluster.text.trim().is_empty() {
                    continue;
                }

                for (text, mut range) in subdivide(&line, &cluster) {
                    let mut span = element::TSpan::new(text);

                    let x = range.start;
                    if x != pos {
                        span = span.set("x", format!("{}em", (x as f32 * fw).r2p(FP)));
                    }

                    if line.get_cell(x).map(|x| x.width()).unwrap_or(0) > 1 {
                        // Make width invalid to force explicit x position attribute in the next span.
                        // This is needed because characters with width > 1 are not monospaced and can overlap
                        // with the next character.
                        range.end = range.start + 1;
                    }

                    let mut color = if cluster.attrs.reverse() {
                        opt.theme.resolve_bg(cluster.attrs.background())
                    } else {
                        opt.theme.resolve_fg(cluster.attrs.foreground())
                    };

                    if cluster.attrs.intensity() == Intensity::Half {
                        color = opt.theme.bg.interpolate_lab(&color, 0.5);
                    }
                    color.a = 1.0;

                    if color != opt.theme.fg {
                        span = span.set("fill", color.to_hex_string());
                    }

                    if cluster.attrs.intensity() == Intensity::Bold {
                        span = span.set("font-weight", "bold");
                    }

                    if cluster.attrs.italic() {
                        span = span.set("font-style", "italic");
                    }

                    if cluster.attrs.underline() != Underline::None {
                        span = span.set("text-decoration", "underline");
                    } else if cluster.attrs.strikethrough() {
                        span = span.set("text-decoration", "line-through");
                    }

                    if cluster.attrs.underline_color() != ColorAttribute::Default {
                        if let Some(mut color) = opt.theme.resolve(cluster.attrs.underline_color())
                        {
                            color.a = 1.0;
                            span = span.set("text-decoration-color", color.to_hex_string());
                        }
                    }

                    if cluster.attrs.underline() != Underline::None {
                        span = span.set(
                            "text-decoration-style",
                            match cluster.attrs.underline() {
                                Underline::Single => "solid",
                                Underline::Double => "double",
                                Underline::Curly => "wavy",
                                Underline::Dotted => "dotted",
                                Underline::Dashed => "dashed",
                                Underline::None => "",
                            },
                        );
                    }

                    tl = tl.add(span);
                    pos = x + range.len();
                }
            }

            sl = sl.add(tl);
            group = group.add(sl);
        }

        let doc = Document::new()
            .set("viewBox", r2p((-pad.x, -pad.y, outer.0, outer.1), FP))
            .set("width", outer.0)
            .set("height", outer.1)
            .add(style)
            .add(background)
            .add(group);

        Ok(svg::write(target, &doc)?)
    }
}

impl Render for SvgRenderer {
    fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()> {
        Self::render(self, surface, target)
    }
}

// ---

fn r2p<T: RoundToPrecision>(value: T, precision: u8) -> T {
    value.r2p(precision)
}

// ---

trait RoundToPrecision {
    fn r2p(&self, precision: u8) -> Self;
}

impl RoundToPrecision for f32 {
    fn r2p(&self, precision: u8) -> Self {
        let k = 10.0f32.powf(precision as f32);
        (self * k).round() / k
    }
}

impl RoundToPrecision for (f32, f32) {
    fn r2p(&self, precision: u8) -> Self {
        (r2p(self.0, precision), r2p(self.1, precision))
    }
}

impl RoundToPrecision for (f32, f32, f32, f32) {
    fn r2p(&self, precision: u8) -> Self {
        (
            r2p(self.0, precision),
            r2p(self.1, precision),
            r2p(self.2, precision),
            r2p(self.3, precision),
        )
    }
}

impl RoundToPrecision for Padding {
    fn r2p(&self, precision: u8) -> Self {
        Padding {
            x: r2p(self.x, precision),
            y: r2p(self.y, precision),
        }
    }
}

// ---

fn subdivide<'a>(line: &'a Line, cluster: &'a CellCluster) -> Subclusters<'a> {
    Subclusters {
        line,
        cluster,
        chars: cluster.text.char_indices(),
        cell_range: cluster.first_cell_idx..cluster.first_cell_idx,
        text_range: 0..0,
        next: None,
    }
}

struct Subclusters<'a> {
    line: &'a Line,
    cluster: &'a CellCluster,
    chars: std::str::CharIndices<'a>,
    cell_range: Range<usize>,
    text_range: Range<usize>,
    next: Option<CellRef<'a>>,
}

impl<'a> Subclusters<'a> {
    fn split(&mut self) -> Option<(&'a str, Range<usize>)> {
        if self.text_range.len() == 0 {
            return None;
        }

        let segment = (
            &self.cluster.text[self.text_range.clone()],
            self.cell_range.clone(),
        );
        self.text_range.start = self.text_range.end;
        self.cell_range.start = self.cell_range.end;
        Some(segment)
    }

    fn next_cell(&mut self) -> Option<CellRef<'a>> {
        self.chars
            .next()
            .and_then(|(i, _)| self.line.get_cell(self.cluster.byte_to_cell_idx(i)))
    }
}

impl<'a> Iterator for Subclusters<'a> {
    type Item = (&'a str, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.is_none() {
                self.next = self.next_cell();
            }

            let Some(next) = self.next else {
                return self.split();
            };

            if next.width() > 1 {
                if let Some(segment) = self.split() {
                    return Some(segment);
                }
            }

            self.cell_range.end += next.width();
            self.text_range.end += next.str().len();
            self.next = None;

            if next.width() > 1 {
                return self.split();
            }
        }
    }
}

// ---

mod styles {
    // third-party imports
    use askama::Template;

    #[derive(Template)]
    #[template(path = "styles/screen.css")]
    pub struct Screen<'a> {
        pub font_family: &'a str,
        pub font_size: f32,
        pub fill: &'a str,
    }
}
