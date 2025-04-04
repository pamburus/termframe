// std imports
use std::{
    borrow::Cow,
    cmp::{max, min},
    collections::{BTreeMap, HashSet},
    ops::{Range, RangeInclusive},
    rc::Rc,
};

// third-party imports
use askama::Template;
use csscolorparser::Color;
use svg::{Document, Node, node::element};
use termwiz::{
    cell::{CellAttributes, Intensity, Underline},
    cellcluster::CellCluster,
    color::{ColorAttribute, SrgbaTuple},
    surface::{Line, Surface, line::CellRef},
};

// local imports
use super::{FontFace, FontStyle, FontWeight, Padding, Render, Theme};
use crate::config::{
    types::Number,
    winstyle::{
        LineCap, WindowButton, WindowButtonIconKind, WindowButtonShape, WindowButtonsPosition,
    },
};

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

        let fp = cfg.rendering.svg.precision; // floating point precision
        let lh = cfg.rendering.line_height.r2p(fp); // line height in em
        let fw = opt.font.metrics.width.r2p(fp); // font width in em
        let dimensions = surface.dimensions(); // surface dimensions in cells
        let size = (
            // terminal surface size in em
            (dimensions.0 as f32 * fw).r2p(fp),
            (dimensions.1 as f32 * lh).r2p(fp),
        );
        let pad = cfg.padding.resolve().r2p(fp); // padding in pixels
        let tyo = ((lh + opt.font.metrics.descender + opt.font.metrics.ascender) / 2.0).r2p(fp); // text y-offset in em

        let mut palette = PaletteBuilder::new(
            bg.clone(),
            fg.clone(),
            opt.theme.clone(),
            cfg.rendering.svg.var_palette,
        );

        let background = element::Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", palette.bg(ColorAttribute::Default));

        let mut used_font_faces = HashSet::new();

        let mut group = element::Group::new();

        let default_weight = opt.font.weights.normal;
        if default_weight != FontWeight::Normal {
            group = group.set("font-weight", svg_weight(default_weight));
        }

        let resolve_fg = |palette: &mut PaletteBuilder, attrs: &CellAttributes| {
            let color = attrs.foreground();
            if cfg.rendering.bold_is_bright && attrs.intensity() == Intensity::Bold {
                palette.bright_fg(color)
            } else {
                palette.fg(color)
            }
        };

        let resolve_bg = |palette: &mut PaletteBuilder, attrs: &CellAttributes| {
            if attrs.reverse() {
                Some(resolve_fg(palette, attrs))
            } else {
                let bg = attrs.background();
                if bg == ColorAttribute::Default {
                    None
                } else {
                    Some(palette.bg(bg))
                }
            }
        };

        let lines = surface.screen_lines();

        let shapes = super::tracing::trace(dimensions.0, dimensions.1, |x, y| {
            resolve_bg(&mut palette, lines[y].get_cell(x)?.attrs())
        });

        let mut bg_group = element::Group::new();
        if let Some(stroke) = opt.settings.rendering.svg.stroke {
            bg_group = bg_group.set("stroke-width", stroke.r2p(fp));
        }

        for shape in shapes {
            let mut d = String::new();

            for contour in &shape.path {
                if !d.is_empty() {
                    d.push(' ');
                }

                build_svg_path(&mut d, contour, lh, fw, fp);
            }

            let color = shape.key;
            let mut path = element::Path::new().set("fill", color.clone()).set("d", d);
            if cfg.rendering.svg.stroke.is_some() {
                path = path.set("stroke", color);
            }

            bg_group = bg_group.add(path);
        }

        group = group.add(
            container()
                .set("viewBox", format!("0 0 {w} {h}", w = size.0, h = size.1))
                .set("width", format!("{}em", size.0))
                .set("height", format!("{}em", size.1))
                .add(bg_group),
        );

        for (row, line) in lines.iter().enumerate() {
            if line.is_whitespace() {
                continue;
            }

            let mut sl = container()
                .set("y", format!("{}em", (row as f32 * lh).r2p(fp)))
                .set("width", format!("{}em", size.0))
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

                    let color = if cluster.attrs.reverse() {
                        palette.bg(cluster.attrs.background())
                    } else {
                        resolve_fg(&mut palette, &cluster.attrs)
                    };

                    if cluster.attrs.intensity() == Intensity::Half
                        && cfg.rendering.faint_opacity.f32() < 1.0
                    {
                        span = span.set("opacity", cfg.rendering.faint_opacity.r2p(fp));
                    }

                    if color != ColorStyleId::DefaultForeground {
                        span = span.set("fill", color);
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

        let content = container()
            .set("x", format!("{}em", pad.left))
            .set("y", format!("{}em", pad.top))
            .set("fill", palette.fg(ColorAttribute::Default))
            .add(group);

        let width = (size.0 + pad.left + pad.right).r2p(fp);
        let height = (size.1 + pad.top + pad.bottom).r2p(fp);

        let font_family_list = opt.font.family.join(", ");

        let class = "terminal";
        let mut screen = element::SVG::new()
            .set("width", format!("{}em", width))
            .set("height", format!("{}em", height))
            .set("font-size", opt.font.size.r2p(fp))
            .set("font-family", font_family_list);
        if !cfg.window.enabled {
            screen = screen.add(background)
        }
        screen = screen.add(content).set("class", class);

        let mut doc = if cfg.window.enabled {
            let width = (opt.font.size * width).r2p(fp);
            let height = (opt.font.size * height).r2p(fp);

            let mut screen = screen.set("y", opt.window.header.height.r2p(fp));
            screen.unassign("xmlns");

            make_window(opt, width, height, screen)
        } else {
            screen
        };

        let mut ss = Default::default();

        let palette = palette.template(class);
        if !palette.vars.is_empty() {
            ss = palette.render()?;
        }

        let faces = collect_font_faces(opt, used_font_faces)?;
        if !faces.is_empty() {
            if !ss.is_empty() {
                ss += "\n";
            }
            ss += &faces.join("\n");
        }

        let style = element::Style::new(ss);
        doc = doc.add(style);

        Ok(svg::write(target, &doc)?)
    }
}

fn build_svg_path(d: &mut String, contour: &[(i32, i32)], lh: f32, fw: f32, fp: u8) {
    let fx = |x| (x as f32 * fw).r2p(fp);
    let fy = |y| (y as f32 * lh).r2p(fp);

    let mut prev = None;
    for &(x, y) in contour {
        match prev {
            Some((px, py)) => {
                if x == px {
                    d.push_str(&format!("V{} ", fy(y)));
                } else if y == py {
                    d.push_str(&format!("H{} ", fx(x)));
                } else {
                    d.push_str(&format!("{},{} ", fx(x), fy(y),));
                }
            }
            None => {
                d.push_str(&format!("M{},{} ", fx(x), fy(y)));
            }
        }
        prev = Some((x, y));
    }
    d.push('Z');
}

fn container() -> element::SVG {
    let mut container = element::SVG::new();
    container.unassign("xmlns");
    container
}

fn make_window(opt: &Options, width: f32, height: f32, screen: element::SVG) -> element::SVG {
    let cfg = &opt.settings;
    let fp = cfg.rendering.svg.precision; // floating point precision
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

    // shadow
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
        );
    if let Some(border) = &header.border {
        window = window.add(
            element::Line::new()
                .set("x1", "0")
                .set("x2", width)
                .set("y1", header.height.r2p(fp))
                .set("y2", header.height.r2p(fp))
                .set("stroke", border.color.resolve(opt.mode).to_hex_string())
                .set("stroke-width", border.width.r2p(fp)),
        );
    }

    let hh2 = (opt.window.header.height / 2.0).r2p(fp);

    // title
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
    window = window.add(make_buttons(opt, width));

    // screen
    window = window.add(screen);

    // frame border
    let gap = border.width + border.gap.unwrap_or_default();
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
                .set("width", (width - gap * 2.0).r2p(fp))
                .set("height", (height - gap * 2.0).r2p(fp))
                .set("x", gap.r2p(fp))
                .set("y", gap.r2p(fp))
                .set("fill", "none")
                .set(
                    "stroke",
                    border.colors.inner.resolve(opt.mode).to_hex_string(),
                )
                .set("stroke-width", border.width.r2p(fp))
                .set("rx", (border.radius - gap).r2p(fp))
                .set("ry", (border.radius - gap).r2p(fp)),
        );

    Document::new()
        .set("width", (width + margin.left + margin.right).r2p(fp))
        .set("height", (height + margin.top + margin.bottom).r2p(fp))
        .add(window)
}

fn make_buttons(opt: &Options, width: f32) -> element::Group {
    let cfg = &opt.window.buttons;
    let fp = opt.settings.rendering.svg.precision; // floating point precision

    let (x, factor) = match cfg.position {
        WindowButtonsPosition::Left => (0.0, 1.0),
        WindowButtonsPosition::Right => (width, -1.0),
    };
    let y = (opt.window.header.height / 2.0).r2p(fp);

    let mut group = element::Group::new();

    for button in &cfg.items {
        let x = (x + factor * button.offset).r2p(fp);

        match &cfg.shape {
            Some(WindowButtonShape::Circle) => {
                let mut shape = element::Circle::new()
                    .set("cx", x)
                    .set("cy", y)
                    .set("r", cfg.size.r2p(fp));
                set_button_style(opt, button, &mut shape);
                group.append(shape);
            }
            Some(WindowButtonShape::Square) => {
                let mut shape = element::Rectangle::new()
                    .set("x", (x - cfg.size / 2.0).r2p(fp))
                    .set("y", (y - cfg.size / 2.0).r2p(fp))
                    .set("width", cfg.size.r2p(fp))
                    .set("height", cfg.size.r2p(fp));
                if let Some(r) = cfg.roundness {
                    shape = shape.set("rx", r.r2p(fp)).set("ry", r.r2p(fp));
                }
                set_button_style(opt, button, &mut shape);
                group.append(shape);
            }
            None => {}
        }

        if let Some(icon) = &button.icon {
            let mut path = match icon.kind {
                WindowButtonIconKind::Close => element::Path::new().set(
                    "d",
                    format!(
                        "M{x1},{y1} {x2},{y2} M{x1},{y2} {x2},{y1}",
                        x1 = (x - icon.size / 2.0).r2p(fp),
                        y1 = (y - icon.size / 2.0).r2p(fp),
                        x2 = (x + icon.size / 2.0).r2p(fp),
                        y2 = (y + icon.size / 2.0).r2p(fp),
                    ),
                ),
                WindowButtonIconKind::Minimize => element::Path::new().set(
                    "d",
                    format!(
                        "M{x1},{y1} {x2},{y1}",
                        x1 = (x - icon.size / 2.0).r2p(fp),
                        y1 = y.r2p(fp),
                        x2 = (x + icon.size / 2.0).r2p(fp),
                    ),
                ),
                WindowButtonIconKind::Maximize => {
                    let r = icon.roundness.map(Number::f32).unwrap_or(2.0);
                    let x1 = (x - icon.size / 2.0).r2p(fp);
                    let x4 = (x + icon.size / 2.0).r2p(fp);
                    let x2 = (x1 + r).r2p(fp);
                    let x3 = (x4 - r).r2p(fp);
                    let y1 = (y - icon.size / 2.0).r2p(fp);
                    let y4 = (y + icon.size / 2.0).r2p(fp);
                    let y2 = (y1 + r).r2p(fp);
                    let y3 = (y4 - r).r2p(fp);

                    element::Path::new().set("d",format!("M{x2},{y1} L{x3},{y1} Q{x4},{y1},{x4},{y2} L{x4},{y3} Q{x4},{y4},{x3},{y4} L{x2},{y4} Q{x1},{y4},{x1},{y3} L{x1},{y2} Q{x1},{y1},{x2},{y1}"))
                }
            };

            path.assign("fill", "none");
            path.assign("stroke", icon.stroke.resolve(opt.mode).to_hex_string());
            if let Some(stroke_width) = icon.stroke_width {
                path.assign("stroke-width", stroke_width.r2p(fp));
            }
            if let Some(linecap) = &icon.stroke_linecap {
                path.assign(
                    "stroke-linecap",
                    match linecap {
                        LineCap::Round => "round",
                        LineCap::Square => "square",
                        LineCap::Butt => "butt",
                    },
                );
            }

            group.append(path);
        }
    }

    group
}

fn set_button_style<N: svg::Node>(opt: &Options, cfg: &WindowButton, node: &mut N) {
    let fp = opt.settings.rendering.svg.precision; // floating point precision

    if let Some(fill) = &cfg.fill {
        node.assign("fill", fill.resolve(opt.mode).to_hex_string());
    } else {
        node.assign("fill", "none");
    }

    if let Some(stroke) = &cfg.stroke {
        node.assign("stroke", stroke.resolve(opt.mode).to_hex_string());
    }

    if let Some(stroke_width) = cfg.stroke_width {
        node.assign("stroke-width", stroke_width.r2p(fp));
    }
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

fn r2p<T: RoundToPrecision>(value: T, precision: u8) -> T::Output {
    value.r2p(precision)
}

// ---

trait RoundToPrecision {
    type Output;

    fn r2p(&self, precision: u8) -> Self::Output;
}

impl RoundToPrecision for f32 {
    type Output = Self;

    fn r2p(&self, precision: u8) -> Self {
        let k = 10.0f32.powf(precision as f32);
        (self * k).round() / k
    }
}

impl RoundToPrecision for Number {
    type Output = f32;

    fn r2p(&self, precision: u8) -> Self::Output {
        r2p(<f32>::from(*self), precision)
    }
}

impl RoundToPrecision for (f32, f32) {
    type Output = Self;

    fn r2p(&self, precision: u8) -> Self {
        (r2p(self.0, precision), r2p(self.1, precision))
    }
}

impl RoundToPrecision for (f32, f32, f32, f32) {
    type Output = Self;

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
    type Output = Self;

    fn r2p(&self, precision: u8) -> Self {
        Padding {
            left: self.left.r2p(precision).into(),
            right: self.right.r2p(precision).into(),
            top: self.top.r2p(precision).into(),
            bottom: self.bottom.r2p(precision).into(),
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

struct PaletteBuilder {
    bg: Color,
    fg: Color,
    theme: Rc<Theme>,
    has_bg: bool,
    has_fg: bool,
    has_br_fg: bool,
    palette: BTreeMap<u8, Color>,
    var_palette: bool,
}

impl PaletteBuilder {
    fn new(bg: Color, fg: Color, theme: Rc<Theme>, var_palette: bool) -> Self {
        Self {
            bg,
            fg,
            theme,
            has_bg: false,
            has_fg: false,
            has_br_fg: false,
            palette: BTreeMap::new(),
            var_palette,
        }
    }

    fn bg(&mut self, attr: ColorAttribute) -> ColorStyle {
        match attr {
            ColorAttribute::Default => {
                if !self.var_palette {
                    return ColorStyle::Custom(self.bg.clone());
                }
                self.has_bg = true;
                ColorStyleId::DefaultBackground.into()
            }
            ColorAttribute::PaletteIndex(i) => {
                let bg = self.bg.clone();
                let color = || self.theme.resolve(attr).unwrap_or(bg);
                if !self.var_palette {
                    return ColorStyle::Custom(color());
                }
                self.palette.entry(i).or_insert_with(color);
                ColorStyleId::Palette(i).into()
            }
            ColorAttribute::TrueColorWithDefaultFallback(c)
            | ColorAttribute::TrueColorWithPaletteFallback(c, _) => Self::custom(c),
        }
    }

    fn fg(&mut self, attr: ColorAttribute) -> ColorStyle {
        match attr {
            ColorAttribute::Default => {
                if !self.var_palette {
                    return ColorStyle::Custom(self.fg.clone());
                }
                self.has_fg = true;
                ColorStyleId::DefaultForeground.into()
            }
            ColorAttribute::PaletteIndex(i) => {
                let fg = self.fg.clone();
                let color = || self.theme.resolve(attr).unwrap_or(fg);
                if !self.var_palette {
                    return ColorStyle::Custom(color());
                }
                self.palette.entry(i).or_insert_with(color);
                ColorStyleId::Palette(i).into()
            }
            ColorAttribute::TrueColorWithDefaultFallback(c)
            | ColorAttribute::TrueColorWithPaletteFallback(c, _) => Self::custom(c),
        }
    }

    fn bright_fg(&mut self, attr: ColorAttribute) -> ColorStyle {
        let attr = match attr {
            ColorAttribute::Default => {
                if !self.var_palette {
                    return ColorStyle::Custom(
                        self.theme.bright_fg.as_ref().unwrap_or(&self.fg).clone(),
                    );
                }
                self.has_br_fg = true;
                return ColorStyleId::BrightForeground.into();
            }
            ColorAttribute::PaletteIndex(i) if i < 8 => ColorAttribute::PaletteIndex(i + 8),
            _ => attr,
        };
        self.fg(attr)
    }

    fn template(&self, name: &str) -> styles::Theme {
        let mut vars = Vec::new();
        if self.has_bg {
            vars.push((
                ColorStyleId::DefaultBackground.name().into(),
                self.bg.to_hex_string(),
            ));
        }
        if self.has_fg {
            vars.push((
                ColorStyleId::DefaultForeground.name().into(),
                self.fg.to_hex_string(),
            ));
        }
        if self.has_br_fg {
            vars.push((
                ColorStyleId::BrightForeground.name().into(),
                self.theme
                    .bright_fg
                    .as_ref()
                    .unwrap_or(&self.fg)
                    .to_hex_string(),
            ));
        }
        for (i, color) in &self.palette {
            vars.push((
                ColorStyleId::Palette(*i).name().into(),
                color.to_hex_string(),
            ));
        }

        styles::Theme {
            name: name.into(),
            vars,
        }
    }

    fn custom(c: SrgbaTuple) -> ColorStyle {
        ColorStyle::Custom(Color::new(c.0, c.1, c.2, c.3))
    }
}

// ---

#[derive(Debug, Clone, PartialEq)]
enum ColorStyle {
    Themed(ColorStyleId),
    Custom(Color),
}

impl ColorStyle {
    fn render(&self) -> Cow<'static, str> {
        match self {
            ColorStyle::Themed(id) => format!("var({id})").into(),
            ColorStyle::Custom(color) => color.to_hex_string().into(),
        }
    }
}

impl PartialEq<ColorStyleId> for ColorStyle {
    fn eq(&self, id: &ColorStyleId) -> bool {
        match self {
            ColorStyle::Themed(i) => i == id,
            _ => false,
        }
    }
}

impl std::fmt::Display for ColorStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl From<ColorStyle> for svg::node::Value {
    fn from(style: ColorStyle) -> Self {
        style.render().as_ref().into()
    }
}

impl From<ColorStyleId> for ColorStyle {
    fn from(id: ColorStyleId) -> Self {
        ColorStyle::Themed(id)
    }
}

// ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorStyleId {
    DefaultBackground,
    DefaultForeground,
    BrightForeground,
    Palette(u8),
}

impl ColorStyleId {
    fn name(&self) -> Cow<'static, str> {
        match self {
            ColorStyleId::DefaultBackground => "--bg".into(),
            ColorStyleId::DefaultForeground => "--fg".into(),
            ColorStyleId::BrightForeground => "--br-fg".into(),
            ColorStyleId::Palette(i) => format!("--c-{}", i).into(),
        }
    }
}

impl std::fmt::Display for ColorStyleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
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

    #[derive(Template)]
    #[template(path = "styles/theme.css")]
    pub struct Theme {
        pub name: String,
        pub vars: Vec<(String, String)>,
    }
}
