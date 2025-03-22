// std imports
use std::{
    cmp::{max, min},
    collections::HashSet,
    ops::{Range, RangeInclusive},
};

// third-party imports
use askama::Template;
use svg::{Document, Node, node::element};
use termwiz::{
    cell::{Intensity, Underline},
    cellcluster::CellCluster,
    color::ColorAttribute,
    surface::{Line, Surface, line::CellRef},
};

// local imports
use super::{FontFace, FontStyle, FontWeight, Padding, Render};

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
        let cfg = &opt.settings;

        let bg = opt.bg();
        let fg = opt.fg();

        let fp = cfg.precision; // floating point precision
        let lh = cfg.line_height.r2p(fp); // line height in em
        let fw = opt.font.metrics.width.r2p(fp); // font width in em
        let dimensions = surface.dimensions(); // surface dimensions in cells
        let size = (
            // terminal surface size in em
            (dimensions.0 as f32 * fw).r2p(fp),
            (dimensions.1 as f32 * lh).r2p(fp),
        );
        let pad = cfg.padding.resolve().r2p(fp); // padding in pixels
        let tyo = ((lh + opt.font.metrics.descender + opt.font.metrics.ascender) / 2.0).r2p(fp); // text y-offset in em

        let background = element::Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", bg.to_hex_string());

        let mut used_font_faces = HashSet::new();

        let mut group = element::Group::new();

        let default_weight = opt.font.weights.normal;
        if default_weight != FontWeight::Normal {
            group = group.set("font-weight", svg_weight(default_weight));
        }

        for (row, line) in surface.screen_lines().iter().enumerate() {
            for cluster in line.cluster(None) {
                let color = if cluster.attrs.reverse() {
                    Some(
                        opt.theme
                            .resolve(cluster.attrs.foreground())
                            .unwrap_or_else(|| fg.clone()),
                    )
                } else {
                    opt.theme.resolve(cluster.attrs.background())
                };

                if let Some(mut color) = color {
                    color.a = 1.0;

                    let x = (cluster.first_cell_idx as f32 * fw - cfg.stroke).r2p(fp);
                    let y = (row as f32 * lh - cfg.stroke).r2p(fp);
                    let width = (cluster.width as f32 * fw + cfg.stroke * 2.0).r2p(fp);
                    let height = (lh + cfg.stroke * 2.0).r2p(fp);

                    let rect = element::Rectangle::new()
                        .set("x", format!("{}em", x))
                        .set("y", format!("{}em", y))
                        .set("width", format!("{}em", width))
                        .set("height", format!("{}em", height))
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
                .set("y", format!("{}em", (row as f32 * lh).r2p(fp)))
                .set("width", format!("{}em", size.0))
                .set("height", format!("{}em", lh))
                .set("overflow", "hidden");
            sl.unassign("xmlns");

            let mut tl = element::Text::new("")
                .set("y", format!("{}em", tyo))
                .set("xml:space", "preserve");

            let mut pos = 0;
            for cluster in line.cluster(None) {
                if cluster.text.trim().is_empty() {
                    continue;
                }

                for (text, mut range) in subdivide(line, &cluster) {
                    if text.trim().is_empty() {
                        continue;
                    }

                    let mut span = element::TSpan::new(text);

                    let mut font_weight = super::FontWeight::Normal;
                    let mut font_style = super::FontStyle::Normal;

                    let x = range.start;
                    if x != pos {
                        span = span.set("x", format!("{}em", (x as f32 * fw).r2p(fp)));
                    }

                    if line.get_cell(x).map(|cell| cell.width()).unwrap_or(0) > 1 {
                        // Make width invalid to force explicit x position attribute in the next span.
                        // This is needed because characters with width > 1 are not monospaced and can overlap
                        // with the next character.
                        range.end = range.start + 1;
                    }

                    let correct = |mut color: ColorAttribute| {
                        if cfg.bold_is_bright && cluster.attrs.intensity() == Intensity::Bold {
                            match color {
                                ColorAttribute::PaletteIndex(i) if i < 8 => {
                                    color = ColorAttribute::PaletteIndex(i + 8)
                                }
                                _ => {}
                            }
                        }
                        color
                    };

                    let mut color = if cluster.attrs.reverse() {
                        opt.theme
                            .resolve(cluster.attrs.background())
                            .unwrap_or_else(|| bg.clone())
                    } else {
                        opt.theme
                            .resolve(correct(cluster.attrs.foreground()))
                            .unwrap_or_else(|| fg.clone())
                    };

                    if cluster.attrs.intensity() == Intensity::Half {
                        color = opt.theme.bg.interpolate_oklab(&color, cfg.faint_opacity);
                    }
                    color.a = 1.0;

                    if color != *fg {
                        span = span.set("fill", color.to_hex_string());
                    }

                    match cluster.attrs.intensity() {
                        Intensity::Normal => {}
                        Intensity::Bold => {
                            let weight = opt.font.weights.bold;
                            if weight != default_weight {
                                span = span.set("font-weight", svg_weight(weight));
                                font_weight = weight;
                            }
                        }
                        Intensity::Half => {
                            let weight = opt.font.weights.faint;
                            if weight != default_weight {
                                span = span.set("font-weight", svg_weight(weight));
                                font_weight = weight;
                            }
                        }
                    }

                    if cluster.attrs.italic() {
                        span = span.set("font-style", "italic");
                        font_style = super::FontStyle::Italic;
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

                    for ch in text.chars() {
                        for (i, font) in opt.font.faces.iter().enumerate().rev() {
                            if match_font_face(font, font_weight, font_style, ch) {
                                if used_font_faces.insert(i) {
                                    log::debug!(
                                        "using font face #{i:02} because it is required at least by character {ch:?} with weight={font_weight:?} style={font_style:?}",
                                    );
                                }
                                break;
                            }
                        }
                    }

                    tl = tl.add(span);
                    pos = x + range.len();
                }
            }

            sl = sl.add(tl);
            group = group.add(sl);
        }

        let mut content = element::SVG::new()
            .set("x", format!("{}em", pad.left))
            .set("y", format!("{}em", pad.top))
            .set("fill", fg.to_hex_string())
            .add(group);
        content.unassign("xmlns");

        let width = (size.0 + pad.left + pad.right).r2p(fp);
        let height = (size.1 + pad.top + pad.bottom).r2p(fp);

        let font_family_list = opt.font.family.join(", ");

        let mut screen = element::SVG::new()
            .set("width", format!("{}em", width))
            .set("height", format!("{}em", height))
            .set("font-size", opt.font.size.r2p(fp))
            .set("font-family", font_family_list);
        if !cfg.window.enabled {
            screen = screen.add(background)
        }
        screen = screen.add(content);

        let mut doc = if cfg.window.enabled {
            let width = (opt.font.size * width).r2p(fp);
            let height = (opt.font.size * height).r2p(fp);

            let mut screen = screen.set("y", opt.window.header.height.r2p(fp));
            screen.unassign("xmlns");

            make_window(opt, width, height, screen)
        } else {
            screen
        };

        let faces = collect_font_faces(opt, used_font_faces)?;
        if !faces.is_empty() {
            let style = element::Style::new(faces.join("\n"));
            doc = doc.add(style);
        }

        Ok(svg::write(target, &doc)?)
    }
}

fn make_window(opt: &Options, width: f32, height: f32, screen: element::SVG) -> element::SVG {
    let cfg = &opt.settings;
    let fp = cfg.precision; // floating point precision
    let margin = cfg
        .window
        .margin
        .unwrap_or(opt.window.margin)
        .resolve()
        .r2p(fp); // margin in pixels
    let height = (height + opt.window.header.height).r2p(fp);
    let border = &opt.window.border;

    let mut window = element::Group::new().set(
        "transform",
        format!("translate({mx},{my})", mx = margin.left, my = margin.top),
    );

    if cfg.window.shadow && opt.window.shadow.enabled {
        let shadow = &opt.window.shadow;
        window = window
            .add(element::Filter::new().set("id", "shadow").add(
                element::FilterEffectGaussianBlur::new().set("stdDeviation", shadow.blur.r2p(fp)),
            ))
            .add(
                element::Rectangle::new()
                    .set("width", width)
                    .set("height", height)
                    .set("x", (shadow.x).r2p(fp))
                    .set("y", (shadow.y).r2p(fp))
                    .set("fill", shadow.color.resolve(opt.mode).to_hex_string())
                    .set("rx", border.radius.r2p(fp))
                    .set("ry", border.radius.r2p(fp))
                    .set("filter", "url(#shadow)"),
            )
    }

    // background
    window = window.add(
        element::Rectangle::new()
            .set("fill", opt.bg().to_hex_string())
            .set("rx", border.radius.r2p(fp))
            .set("ry", border.radius.r2p(fp))
            .set("width", width)
            .set("height", height),
    );

    // header
    let header = &opt.window.header;
    window = window
        .add(
            element::ClipPath::new().set("id", "header").add(
                element::Rectangle::new()
                    .set("width", width)
                    .set("height", header.height.r2p(fp)),
            ),
        )
        .add(
            element::Rectangle::new()
                .set("fill", header.color.resolve(opt.mode).to_hex_string())
                .set("rx", border.radius.r2p(fp))
                .set("ry", border.radius.r2p(fp))
                .set("width", width)
                .set("height", 2.0 * header.height.r2p(fp))
                .set("clip-path", "url(#header)"),
        )
        .add(
            element::Line::new()
                .set("x1", "0")
                .set("x2", width)
                .set("y1", header.height.r2p(fp))
                .set("y2", header.height.r2p(fp))
                .set(
                    "stroke",
                    opt.window
                        .border
                        .colors
                        .outer
                        .resolve(opt.mode)
                        .to_hex_string(),
                )
                .set("stroke-width", border.width.r2p(fp)),
        );

    let hh2 = (opt.window.header.height / 2.0).r2p(fp);
    let r = opt.window.buttons.radius.r2p(fp);
    let sp = opt.window.buttons.spacing.r2p(fp);
    let buttons = &opt.window.buttons;

    if let Some(title) = &opt.title {
        let cfg = &opt.window.title;
        let mut title = element::Text::new(title)
            .set("x", (width / 2.0).r2p(fp))
            .set("y", (hh2).r2p(fp))
            .set("fill", cfg.color.resolve(opt.mode).to_hex_string())
            .set("font-size", cfg.font.size.r2p(fp))
            .set("font-family", cfg.font.family.join(", "))
            .set("text-anchor", "middle")
            .set("dominant-baseline", "central");
        if let Some(weight) = &cfg.font.weight {
            title = title.set("font-weight", weight.as_str())
        }
        window = window.add(title);
    }

    // buttons
    window = window
        .add(
            element::Circle::new()
                .set("cx", (hh2).r2p(fp))
                .set("cy", hh2)
                .set("r", r)
                .set(
                    "fill",
                    buttons.close.color.resolve(opt.mode).to_hex_string(),
                ),
        )
        .add(
            element::Circle::new()
                .set("cx", (hh2 + sp).r2p(fp))
                .set("cy", hh2)
                .set("r", r)
                .set(
                    "fill",
                    buttons.minimize.color.resolve(opt.mode).to_hex_string(),
                ),
        )
        .add(
            element::Circle::new()
                .set("cx", (hh2 + sp * 2.0).r2p(fp))
                .set("cy", hh2)
                .set("r", r)
                .set(
                    "fill",
                    buttons.maximize.color.resolve(opt.mode).to_hex_string(),
                ),
        );

    // screen
    window = window.add(screen);

    // frame border
    window = window
        .add(
            element::Rectangle::new()
                .set("width", (width + 0.0).r2p(fp))
                .set("height", (height + 0.0).r2p(fp))
                .set("fill", "none")
                .set(
                    "stroke",
                    border.colors.outer.resolve(opt.mode).to_hex_string(),
                )
                .set("stroke-width", border.width.r2p(fp))
                .set("rx", (border.radius + 0.0).r2p(fp))
                .set("ry", (border.radius + 0.0).r2p(fp)),
        )
        .add(
            element::Rectangle::new()
                .set("width", (width - 2.0).r2p(fp))
                .set("height", (height - 2.0).r2p(fp))
                .set("x", (1.0).r2p(fp))
                .set("y", (1.0).r2p(fp))
                .set("fill", "none")
                .set(
                    "stroke",
                    border.colors.inner.resolve(opt.mode).to_hex_string(),
                )
                .set("stroke-width", border.width.r2p(fp))
                .set("rx", (border.radius - 1.0).r2p(fp))
                .set("ry", (border.radius - 1.0).r2p(fp)),
        );

    Document::new()
        .set("width", (width + margin.left + margin.right).r2p(fp))
        .set("height", (height + margin.top + margin.bottom).r2p(fp))
        .add(window)
}

fn collect_font_faces(opt: &Options, used_font_faces: HashSet<usize>) -> Result<Vec<String>> {
    let faces = &opt
        .font
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| used_font_faces.contains(i))
        .map(|(_, face)| styles::FontFace {
            font_family: face.family.clone(),
            font_weight: match face.weight {
                FontWeight::Normal => "normal".into(),
                FontWeight::Bold => "bold".into(),
                FontWeight::Fixed(w) => w.to_string(),
                FontWeight::Variable(min, max) => {
                    format!("{min} {max}", min = f32::from(min), max = f32::from(max))
                }
            },
            font_style: face.style.map(|style| match style {
                FontStyle::Normal => "normal".into(),
                FontStyle::Italic => "italic".into(),
                FontStyle::Oblique => "oblique".into(),
            }),
            src_url: face.url.to_string(),
            format: face.format.map(|f| f.css()),
        })
        .collect::<Vec<_>>();

    let faces = faces
        .iter()
        .map(|face| {
            face.render()
                .map_err(Into::into)
                .map(|x| x.trim().to_owned())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(faces)
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
            left: r2p(self.left, precision),
            right: r2p(self.right, precision),
            top: r2p(self.top, precision),
            bottom: r2p(self.bottom, precision),
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
        if self.text_range.is_empty() {
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

trait Intersection {
    fn intersection(&self, other: &Self) -> Option<Self>
    where
        Self: Sized;
}

impl<T> Intersection for RangeInclusive<T>
where
    T: Ord + Clone + Copy,
{
    fn intersection(&self, other: &Self) -> Option<Self>
    where
        Self: Sized,
    {
        let start = max(*self.start(), *other.start());
        let end = min(*self.end(), *other.end());

        if start <= end {
            Some(start..=end)
        } else {
            None
        }
    }
}

// ---

fn match_font_face(face: &FontFace, weight: FontWeight, style: FontStyle, ch: char) -> bool {
    let target: (u16, u16) = match weight {
        FontWeight::Normal => (400, 400),
        FontWeight::Bold => (600, 600),
        FontWeight::Fixed(w) => (w, w),
        FontWeight::Variable(min, max) => (min, max),
    };
    let target = RangeInclusive::new(target.0, target.1);

    let range = face.weight.range();
    let range = RangeInclusive::new(range.0, range.1);

    if range.intersection(&target).is_none() {
        return false;
    }

    if let Some(face_style) = &face.style {
        if *face_style != style {
            return false;
        }
    }

    face.chars.has_char(ch)
}

fn svg_weight(weight: FontWeight) -> String {
    match weight {
        FontWeight::Normal => "normal".into(),
        FontWeight::Bold => "bold".into(),
        FontWeight::Fixed(w) => w.to_string(),
        FontWeight::Variable(_, max) => max.to_string(),
    }
}

// ---

mod styles {
    // third-party imports
    use askama::Template;

    #[derive(Template)]
    #[template(path = "styles/font-face.css")]
    pub struct FontFace {
        pub font_family: String,
        pub font_weight: String,
        pub font_style: Option<String>,
        pub src_url: String,
        pub format: Option<&'static str>,
    }
}
