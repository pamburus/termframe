use std::{
    borrow::Cow,
    cmp::{max, min},
    collections::{BTreeMap, HashSet},
    ops::{Range, RangeInclusive},
    rc::Rc,
};

use askama::Template;
use csscolorparser::Color;
use indexmap::IndexSet;
use svg::{Document, Node, node::element};
use termwiz::{
    cell::{CellAttributes, Intensity, Underline},
    cellcluster::CellCluster,
    color::{ColorAttribute, SrgbaTuple},
    surface::{Line, Surface, line::CellRef},
};

use super::{FontFace, FontStyle, FontWeight, Padding, Render, Theme};
use crate::config::{
    types::Number,
    winstyle::{
        LineCap, WindowButton, WindowButtonIconKind, WindowButtonShape, WindowButtonsPosition,
    },
};

pub use super::{Options, Result};

/// A renderer for generating SVG representations of terminal surfaces.
pub struct SvgRenderer {
    options: Rc<Options>,
}

impl SvgRenderer {
    /// Creates a new `SvgRenderer` with the given options.
    pub fn new(options: Rc<Options>) -> Self {
        Self { options }
    }

    /// Renders the given terminal surface to the specified target as an SVG.
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

        let mut unresolved = IndexSet::new();

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

                for (text, mut range) in subdivide(line, &cluster, opt) {
                    if text.trim().is_empty() {
                        continue;
                    }

                    let mut span = element::TSpan::new(text);

                    let x = range.start;
                    if x != pos {
                        span.assign("x", format!("{}em", (x as f32 * fw).r2p(fp)));
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
                        span.assign("opacity", cfg.rendering.faint_opacity.r2p(fp));
                    }

                    if color != ColorStyleId::DefaultForeground {
                        span.assign("fill", color);
                    }

                    let (weight, style) = font_params(&cluster.attrs, opt);

                    if weight != default_weight {
                        span.assign("font-weight", svg_weight(weight));
                    }

                    match style {
                        FontStyle::Normal => {}
                        FontStyle::Italic => {
                            span.assign("font-style", "italic");
                        }
                        FontStyle::Oblique => {
                            span.assign("font-style", "oblique");
                        }
                    }

                    if cluster.attrs.underline() != Underline::None {
                        span.assign("text-decoration", "underline");
                    } else if cluster.attrs.strikethrough() {
                        span.assign("text-decoration", "line-through");
                    }

                    if cluster.attrs.underline_color() != ColorAttribute::Default {
                        if let Some(mut color) = opt.theme.resolve(cluster.attrs.underline_color())
                        {
                            color.a = 1.0;
                            span.assign("text-decoration-color", color.to_hex_string());
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

                    let mut text_length_needed = false;

                    for ch in text.chars() {
                        if let Some(i) = find_matching_font(ch, weight, style, opt) {
                            log::trace!(
                                "character {ch:>8?} with weight={weight:>8?} style={style:>8?}: requires font #{i:02}"
                            );
                            if used_font_faces.insert(i) {
                                log::debug!(
                                    "using font face #{i:02} because it is required at least by character {ch:?} with weight={weight:?} style={style:?}",
                                );
                            }
                            if !opt.font.faces[i].metrics_match {
                                text_length_needed = true;
                            }
                        } else {
                            unresolved.insert(ch);
                        }
                    }

                    if text_length_needed {
                        sl.append(tl);
                        sl.append(
                            element::Text::new("")
                                .set("x", format!("{}em", (x as f32 * fw).r2p(fp)))
                                .set("y", format!("{}em", tyo))
                                .set("xml:space", "preserve")
                                .set(
                                    "textLength",
                                    format!("{}em", (range.len() as f32 * fw).r2p(fp)),
                                )
                                .add(span),
                        );
                        pos = x + range.len();
                        tl = element::Text::new("")
                            .set("x", format!("{}em", (pos as f32 * fw).r2p(fp)))
                            .set("y", format!("{}em", tyo))
                            .set("xml:space", "preserve");
                    } else {
                        tl = tl.add(span);
                        pos = x + range.len();
                    }
                }
            }

            sl = sl.add(tl);
            group = group.add(sl);
        }

        for ch in unresolved {
            log::warn!("font not found for character {ch:2} ({ch:?})");
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

/// Builds an SVG path string from a contour.
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

/// Creates a new SVG container element.
fn container() -> element::SVG {
    let mut container = element::SVG::new();
    container.unassign("xmlns");
    container
}

/// Creates an SVG representation of a window with the given options.
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

/// Creates the window buttons for the SVG representation.
///
/// # Arguments
///
/// * `opt` - A reference to the `Options` struct containing configuration settings.
/// * `width` - The width of the window.
///
/// # Returns
///
/// A `Group` element containing the window buttons.
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

/// Sets the style for a window button.
///
/// # Arguments
///
/// * `opt` - A reference to the `Options` struct containing configuration settings.
/// * `cfg` - A reference to the `WindowButton` struct containing button settings.
/// * `node` - A mutable reference to the SVG node to apply the style to.
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

/// Collects the font faces used in the SVG representation.
///
/// # Arguments
///
/// * `opt` - A reference to the `Options` struct containing configuration settings.
/// * `used_font_faces` - A `HashSet` containing the indices of the used font faces.
///
/// # Returns
///
/// A `Result` containing a vector of strings representing the font faces.
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

/// Rounds a value to the specified precision.
///
/// # Arguments
///
/// * `value` - The value to round.
/// * `precision` - The number of decimal places to round to.
///
/// # Returns
///
/// The rounded value.
fn r2p<T: RoundToPrecision>(value: T, precision: u8) -> T::Output {
    value.r2p(precision)
}

// ---

/// A trait for rounding values to a specified precision.
trait RoundToPrecision {
    type Output;

    /// Rounds the value to the specified precision.
    ///
    /// # Arguments
    ///
    /// * `precision` - The number of decimal places to round to.
    ///
    /// # Returns
    ///
    /// The rounded value.
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

/// Determines the font weight and style based on cell attributes.
///
/// # Arguments
///
/// * `attrs` - A reference to the `CellAttributes` struct containing cell attributes.
/// * `opt` - A reference to the `Options` struct containing configuration settings.
///
/// # Returns
///
/// A tuple containing the font weight and style.
fn font_params(attrs: &CellAttributes, opt: &Options) -> (FontWeight, FontStyle) {
    let weight = match attrs.intensity() {
        Intensity::Normal => opt.font.weights.normal,
        Intensity::Bold => opt.font.weights.bold,
        Intensity::Half => opt.font.weights.faint,
    };

    let style = if attrs.italic() {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };

    (weight, style)
}

/// Finds a matching font for a given character, weight, and style.
///
/// # Arguments
///
/// * `ch` - The character to find a matching font for.
/// * `weight` - The font weight.
/// * `style` - The font style.
/// * `opt` - A reference to the `Options` struct containing configuration settings.
///
/// # Returns
///
/// An `Option` containing the index of the matching font face, or `None` if no match is found.
fn find_matching_font(
    ch: char,
    weight: FontWeight,
    style: FontStyle,
    opt: &Options,
) -> Option<usize> {
    for (i, font) in opt.font.faces.iter().enumerate().rev() {
        if match_font_face(font, Some(weight), Some(style), ch) {
            return Some(i);
        }
    }

    for (i, font) in opt.font.faces.iter().enumerate().rev() {
        if match_font_face(font, None, Some(style), ch) {
            return Some(i);
        }
    }

    for (i, font) in opt.font.faces.iter().enumerate().rev() {
        if match_font_face(font, None, None, ch) {
            return Some(i);
        }
    }

    None
}

// ---

/// Subdivides a cell cluster into subclusters based on font parameters.
///
/// # Arguments
///
/// * `line` - A reference to the `Line` struct containing the line of cells.
/// * `cluster` - A reference to the `CellCluster` struct containing the cell cluster.
/// * `opt` - A reference to the `Options` struct containing configuration settings.
///
/// # Returns
///
/// A `Subclusters` iterator for iterating over the subclusters.
fn subdivide<'a>(line: &'a Line, cluster: &'a CellCluster, opt: &'a Options) -> Subclusters<'a> {
    let (weight, style) = font_params(&cluster.attrs, opt);

    Subclusters {
        line,
        cluster,
        opt,
        chars: cluster.text.char_indices(),
        cell_range: cluster.first_cell_idx..cluster.first_cell_idx,
        text_range: 0..0,
        weight,
        style,
        next: None,
        font: None,
    }
}

/// An iterator for iterating over subclusters of a cell cluster.
struct Subclusters<'a> {
    line: &'a Line,
    cluster: &'a CellCluster,
    opt: &'a Options,
    chars: std::str::CharIndices<'a>,
    cell_range: Range<usize>,
    text_range: Range<usize>,
    weight: FontWeight,
    style: FontStyle,
    next: Option<CellRef<'a>>,
    font: Option<usize>,
}

impl<'a> Subclusters<'a> {
    /// Splits the current subcluster and returns the text and cell range.
    ///
    /// # Returns
    ///
    /// An `Option` containing a tuple with the text and cell range of the subcluster.
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

    /// Retrieves the next cell in the line.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the next cell.
    fn next_cell(&mut self) -> Option<CellRef<'a>> {
        self.chars
            .next()
            .and_then(|(i, _)| self.line.get_cell(self.cluster.byte_to_cell_idx(i)))
    }
}
impl<'a> Iterator for Subclusters<'a> {
    type Item = (&'a str, Range<usize>);

    /// Advances the iterator and returns the next subcluster.
    ///
    /// Returns `None` when iteration is finished.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.is_none() {
                self.next = self.next_cell();
            }

            let Some(next) = self.next else {
                return self.split();
            };

            let ch = next.str().chars().next();
            let font = ch.and_then(|ch| find_matching_font(ch, self.weight, self.style, self.opt));
            let old_font = std::mem::replace(&mut self.font, font);

            let old_mm = old_font
                .map(|i| self.opt.font.faces[i].metrics_match)
                .unwrap_or_default();

            let new_mm = self
                .font
                .map(|i| self.opt.font.faces[i].metrics_match)
                .unwrap_or_default();

            let split = next.width() > 1 || (old_font != self.font && !(old_mm && new_mm));

            log::trace!(
                "char={ch:?} old-font={old_font:?} new-font={new_font:?} old-mm={old_mm} new-mm={new_mm} width={width} split={split}",
                new_font = self.font,
                width = next.width(),
            );

            if split {
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
    /// Computes the intersection of two ranges.
    ///
    /// Returns `None` if the ranges do not overlap.
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
    /// Creates a new `PaletteBuilder`.
    ///
    /// # Arguments
    ///
    /// * `bg` - The background color.
    /// * `fg` - The foreground color.
    /// * `theme` - The theme to use.
    /// * `var_palette` - Whether to use a variable palette.
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

    /// Resolves the background color style and adds it to the palette.
    ///
    /// # Arguments
    ///
    /// * `attr` - The color attribute.
    ///
    /// # Returns
    ///
    /// The resolved background color style.
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

    /// Resolves the foreground color style and adds it to the palette.
    ///
    /// # Arguments
    ///
    /// * `attr` - The color attribute.
    ///
    /// # Returns
    ///
    /// The resolved foreground color style.
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

    /// Resolves the bright foreground color style and adds it to the palette.
    ///
    /// # Arguments
    ///
    /// * `attr` - The color attribute.
    ///
    /// # Returns
    ///
    /// The resolved bright foreground color style.
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

    /// Generates a CSS template for the theme containing built palette colors.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the theme.
    ///
    /// # Returns
    ///
    /// The generated CSS template.
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

    /// Creates a custom color style.
    ///
    /// # Arguments
    ///
    /// * `c` - The color tuple.
    ///
    /// # Returns
    ///
    /// The custom color style.
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
    /// Renders the color style as a string.
    ///
    /// # Returns
    ///
    /// The rendered color style.
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
    /// Returns the name of the color style ID.
    ///
    /// # Returns
    ///
    /// The name of the color style ID.
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

/// Matches a font face based on weight, style, and character.
///
/// # Arguments
///
/// * `face` - The font face to match.
/// * `weight` - The font weight.
/// * `style` - The font style.
/// * `ch` - The character to match.
///
/// # Returns
///
/// `true` if the font face matches, `false` otherwise.
fn match_font_face(
    face: &FontFace,
    weight: Option<FontWeight>,
    style: Option<FontStyle>,
    ch: char,
) -> bool {
    if let Some(weight) = weight {
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
    }

    if let Some(style) = style {
        if let Some(face_style) = &face.style {
            if *face_style != style {
                return false;
            }
        }
    }

    face.chars.has_char(ch)
}

/// Converts a font weight to an SVG-compatible string.
///
/// # Arguments
///
/// * `weight` - The font weight.
///
/// # Returns
///
/// The SVG-compatible string representation of the font weight.
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
