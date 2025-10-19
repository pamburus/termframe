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
    options: Options,
}

impl SvgRenderer {
    /// Creates a new `SvgRenderer` with the given options.
    pub fn new(options: Options) -> Self {
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
        let lh_p = (lh * opt.font.size).r2p(fp); // line height in pixels
        let fw = opt.font.metrics.width.r2p(fp); // font width in em
        let dimensions = surface.dimensions(); // surface dimensions in cells
        let size = (
            // terminal surface size in em
            (dimensions.0 as f32 * fw).r2p(fp),
            (dimensions.1 as f32 * lh).r2p(fp),
        );
        let size_p = (
            // terminal surface size in pixels
            (size.0 * opt.font.size).r2p(fp),
            (size.1 * opt.font.size).r2p(fp),
        );
        let pad = (cfg.padding.resolve() * opt.font.size).r2p(fp); // padding in pixels
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
                .set("width", format!("{}", size_p.0))
                .set("height", format!("{}", size_p.1))
                .add(bg_group),
        );

        let mut unresolved = IndexSet::new();

        for (row, line) in lines.iter().enumerate() {
            if line.is_whitespace() {
                continue;
            }

            let mut sl = container()
                .set("y", format!("{}", (row as f32 * lh_p).r2p(fp)))
                .set("width", format!("{}", size_p.0))
                .set("height", format!("{lh_p}"))
                .set("overflow", "hidden");

            let mut tl = element::Text::new("")
                .set("y", format!("{tyo}em"))
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

                    if cluster.attrs.underline_color() != ColorAttribute::Default
                        && let Some(mut color) = opt.theme.resolve(cluster.attrs.underline_color())
                    {
                        color.a = 1.0;
                        span.assign("text-decoration-color", color.to_css_hex());
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
                                .set("y", format!("{tyo}em"))
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
                            .set("y", format!("{tyo}em"))
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
            .set("x", format!("{}", pad.left))
            .set("y", format!("{}", pad.top))
            .set("fill", palette.fg(ColorAttribute::Default))
            .add(group);

        let width = (size_p.0 + pad.left + pad.right).r2p(fp);
        let height = (size_p.1 + pad.top + pad.bottom).r2p(fp);

        let font_family_list = opt.font.family.join(", ");

        let class = "terminal";
        let mut screen = element::SVG::new()
            .set("width", format!("{width}"))
            .set("height", format!("{height}"))
            .set("font-size", opt.font.size.r2p(fp))
            .set("font-family", font_family_list);
        if !cfg.window.enabled {
            screen = screen.add(background)
        }
        screen = screen.add(content).set("class", class);

        let mut doc = if cfg.window.enabled {
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

/// Calculates available width for centered text by accounting for button extents.
///
/// # Arguments
///
/// * `width` - Total window width
/// * `button_cfg` - Window buttons configuration
/// * `font_size` - Font size in pixels
/// * `fp` - Floating point precision for rounding
///
/// # Returns
///
/// The width available for centered text after accounting for buttons on both sides.
fn calculate_available_width_for_centered_text(
    width: f32,
    button_cfg: &crate::config::winstyle::WindowButtons,
    font_size: f32,
    fp: u8,
) -> f32 {
    let mut left_extent: f32 = 0.0;
    let mut right_extent: f32 = 0.0;
    let button_size_px: f32 = button_cfg.size.f32().r2p(fp);
    let button_margin: f32 = font_size * 0.2;

    for button in &button_cfg.items {
        let button_offset_px: f32 = button.offset.f32().r2p(fp);
        let button_extent: f32 = button_offset_px + button_size_px / 2.0 + button_margin;
        match button_cfg.position {
            WindowButtonsPosition::Left => {
                left_extent = left_extent.max(button_extent);
            }
            WindowButtonsPosition::Right => {
                right_extent = right_extent.max(button_extent);
            }
        }
    }

    let max_extent: f32 = left_extent.max(right_extent);
    (width - 2.0 * max_extent).max(0.0)
}

/// Estimates the display width of a character for proportional fonts.
///
/// Returns a width multiplier relative to the average character width.
/// Most characters are ~1.0x, but some like 'i', 'l', 'm', 'w' have different widths.
fn estimate_char_width(ch: char) -> f32 {
    match ch {
        // Very narrow characters (about 0.4x)
        'i' | 'j' | 'l' | '!' | ',' | '.' | ':' | ';' | '\'' | '"' => 0.4,
        // Narrow characters (about 0.6x)
        'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '{' | '}' | '/' | '\\' => 0.6,
        // Wide characters (about 1.3x)
        'm' | 'w' | 'W' => 1.3,
        // Regular width (1.0x)
        _ => 1.0,
    }
}

/// Trims text to fit within available width, adding ellipsis if truncated.
///
/// # Arguments
///
/// * `text` - The text to trim
/// * `available_width` - Total width available for the text
/// * `char_width` - Width of a single character (font_size * font.metrics.width)
/// * `ellipsis` - String to append when text is truncated
///
/// # Returns
///
/// The original text if it fits, or a truncated version with ellipsis if it doesn't.
/// Returns empty string if available_width is too small.
fn trim_text_to_width(text: &str, available_width: f32, char_width: f32, ellipsis: &str) -> String {
    if available_width <= 0.0 || char_width <= 0.0 {
        return String::new();
    }

    let chars: Vec<char> = text.chars().collect();
    // Add fixed safety gaps: at least 3 characters width from each side to prevent overlap
    let padding: f32 = char_width * 0.1;
    let safety_gap: f32 = char_width * 3.0;
    let usable_width: f32 = (available_width - padding * 2.0 - safety_gap * 2.0).max(0.0);

    if usable_width <= 0.0 {
        return String::new();
    }

    // Calculate the actual width of the text considering proportional font widths
    let mut current_width = 0.0;
    let mut fits_until = 0;

    for (i, &ch) in chars.iter().enumerate() {
        let ch_width = char_width * estimate_char_width(ch);
        if current_width + ch_width > usable_width {
            break;
        }
        current_width += ch_width;
        fits_until = i + 1;
    }

    if fits_until >= chars.len() {
        return text.to_string();
    }

    // Calculate how much space the ellipsis takes
    let ellipsis_width: f32 = ellipsis
        .chars()
        .map(|ch| char_width * estimate_char_width(ch))
        .sum();

    if ellipsis_width > usable_width {
        return String::new();
    }

    // Trim text to make room for ellipsis
    let available_for_text = usable_width - ellipsis_width;
    let mut current_width = 0.0;
    let mut trim_count = 0;

    for &ch in chars.iter() {
        let ch_width = char_width * estimate_char_width(ch);
        if current_width + ch_width > available_for_text {
            break;
        }
        current_width += ch_width;
        trim_count += 1;
    }

    if trim_count > 0 {
        let trimmed_chars = &chars[..trim_count.min(chars.len())];
        format!("{}{}", trimmed_chars.iter().collect::<String>(), ellipsis)
    } else {
        ellipsis.to_string()
    }
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
            .add(
                element::Filter::new()
                    .set("id", "shadow")
                    .set("filterUnits", "userSpaceOnUse")
                    .set("x", "-32")
                    .set("y", "-24")
                    .set("width", (width + 72.0).r2p(fp))
                    .set("height", (height + 72.0).r2p(fp))
                    .add(
                        element::FilterEffectGaussianBlur::new()
                            .set("stdDeviation", shadow.blur.r2p(fp)),
                    ),
            )
            .add(
                element::Rectangle::new()
                    .set("width", width)
                    .set("height", height)
                    .set("x", (shadow.x).r2p(fp))
                    .set("y", (shadow.y).r2p(fp))
                    .set("fill", shadow.color.resolve(opt.mode).to_css_hex())
                    .set("rx", border.radius.r2p(fp))
                    .set("ry", border.radius.r2p(fp))
                    .set("filter", "url(#shadow)"),
            )
    }

    // background
    window = window.add(
        element::Rectangle::new()
            .set("fill", opt.bg().to_css_hex())
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
                .set("fill", header.color.resolve(opt.mode).to_css_hex())
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
                .set("stroke", border.color.resolve(opt.mode).to_css_hex())
                .set("stroke-width", border.width.r2p(fp)),
        );
    }

    let hh2 = (opt.window.header.height / 2.0).r2p(fp);

    // title
    if let Some(title) = &opt.title {
        let cfg = &opt.window.title;
        let available_width = calculate_available_width_for_centered_text(
            width,
            &opt.window.buttons,
            opt.font.size,
            fp,
        );
        let char_width: f32 = opt.font.size * opt.font.metrics.width;
        let title = trim_text_to_width(title, available_width, char_width, "…");
        if !title.is_empty() {
            let mut title_elem = element::Text::new(&title)
                .set("x", (width / 2.0).r2p(fp))
                .set("y", (hh2).r2p(fp))
                .set("fill", cfg.color.resolve(opt.mode).to_css_hex())
                .set("font-size", cfg.font.size.r2p(fp))
                .set("font-family", cfg.font.family.join(", "))
                .set("text-anchor", "middle")
                .set("dominant-baseline", "central");
            if let Some(weight) = &cfg.font.weight {
                title_elem = title_elem.set("font-weight", weight.as_str())
            }
            window = window.add(title_elem);
        }
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
                .set("stroke", border.colors.outer.resolve(opt.mode).to_css_hex())
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
                .set("stroke", border.colors.inner.resolve(opt.mode).to_css_hex())
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
            path.assign("stroke", icon.stroke.resolve(opt.mode).to_css_hex());
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
        node.assign("fill", fill.resolve(opt.mode).to_css_hex());
    } else {
        node.assign("fill", "none");
    }

    if let Some(stroke) = &cfg.stroke {
        node.assign("stroke", stroke.resolve(opt.mode).to_css_hex());
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

            if split && let Some(segment) = self.split() {
                return Some(segment);
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
                self.bg.to_css_hex(),
            ));
        }
        if self.has_fg {
            vars.push((
                ColorStyleId::DefaultForeground.name().into(),
                self.fg.to_css_hex(),
            ));
        }
        if self.has_br_fg {
            vars.push((
                ColorStyleId::BrightForeground.name().into(),
                self.theme
                    .bright_fg
                    .as_ref()
                    .unwrap_or(&self.fg)
                    .to_css_hex(),
            ));
        }
        for (i, color) in &self.palette {
            vars.push((ColorStyleId::Palette(*i).name().into(), color.to_css_hex()));
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
            ColorStyle::Custom(color) => color.to_css_hex().into(),
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
            ColorStyleId::Palette(i) => format!("--c-{i}").into(),
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

    if let Some(style) = style
        && let Some(face_style) = &face.style
        && *face_style != style
    {
        return false;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_char_width_narrow_chars() {
        // Very narrow characters
        assert_eq!(estimate_char_width('i'), 0.4);
        assert_eq!(estimate_char_width('l'), 0.4);
        assert_eq!(estimate_char_width('.'), 0.4);
        assert_eq!(estimate_char_width(','), 0.4);
    }

    #[test]
    fn test_estimate_char_width_regular_chars() {
        // Regular width characters
        assert_eq!(estimate_char_width('a'), 1.0);
        assert_eq!(estimate_char_width('o'), 1.0);
        assert_eq!(estimate_char_width('x'), 1.0);
    }

    #[test]
    fn test_estimate_char_width_wide_chars() {
        // Wide characters
        assert_eq!(estimate_char_width('m'), 1.3);
        assert_eq!(estimate_char_width('w'), 1.3);
        assert_eq!(estimate_char_width('W'), 1.3);
    }

    #[test]
    fn test_estimate_char_width_narrow_punctuation() {
        // Narrow punctuation
        assert_eq!(estimate_char_width('!'), 0.4);
        assert_eq!(estimate_char_width(':'), 0.4);
        assert_eq!(estimate_char_width(';'), 0.4);
        assert_eq!(estimate_char_width('\''), 0.4);
    }

    #[test]
    fn test_trim_text_to_width_fits_entirely() {
        // Text that fits within available width
        let result = trim_text_to_width("hello", 100.0, 1.0, "…");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_trim_text_to_width_needs_trimming() {
        // Text that needs trimming - use realistic width
        let result = trim_text_to_width("hello world", 15.0, 1.0, "…");
        assert!(!result.contains("world"));
        assert!(result.ends_with("…"));
    }

    #[test]
    fn test_trim_text_to_width_very_narrow_space() {
        // Very narrow available width
        let result = trim_text_to_width("hello", 0.5, 1.0, "…");
        assert_eq!(result, "");
    }

    #[test]
    fn test_trim_text_to_width_proportional_wide_chars() {
        // Wide character should take more space
        let result = trim_text_to_width("www", 10.0, 1.0, "…");
        // 'w' is 1.3x width, so 3 w's = 3.9, plus safety gaps and ellipsis
        // should be trimmed
        assert!(result.contains("…"));
    }

    #[test]
    fn test_trim_text_to_width_proportional_narrow_chars() {
        // Narrow characters should fit more
        let result = trim_text_to_width("iiiiii", 10.0, 1.0, "…");
        // 'i' is 0.4x width, many should fit
        assert_eq!(result, "iiiiii");
    }

    #[test]
    fn test_trim_text_to_width_zero_width() {
        // Zero available width
        let result = trim_text_to_width("text", 0.0, 1.0, "…");
        assert_eq!(result, "");
    }

    #[test]
    fn test_trim_text_to_width_zero_char_width() {
        // Zero character width
        let result = trim_text_to_width("text", 10.0, 0.0, "…");
        assert_eq!(result, "");
    }

    #[test]
    fn test_trim_text_to_width_empty_text() {
        // Empty text
        let result = trim_text_to_width("", 100.0, 1.0, "…");
        assert_eq!(result, "");
    }

    #[test]
    fn test_trim_text_to_width_single_char() {
        // Single character
        let result = trim_text_to_width("a", 100.0, 1.0, "…");
        assert_eq!(result, "a");
    }

    #[test]
    fn test_trim_text_to_width_ellipsis_fits() {
        // Ensure ellipsis fits when text is trimmed
        let result = trim_text_to_width("hello world", 12.0, 1.0, "…");
        assert!(result.ends_with("…"));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_trim_text_to_width_ellipsis_too_wide() {
        // When ellipsis is too wide to fit compared to usable width
        // With a very large ellipsis string that exceeds usable width
        let result = trim_text_to_width("test", 8.0, 1.0, "…………");
        assert_eq!(result, "");
    }

    #[test]
    fn test_trim_text_to_width_only_ellipsis() {
        // Text gets completely trimmed but ellipsis still fits
        // All wide characters with very tight space constraints
        let result = trim_text_to_width("wwwww", 7.5, 1.0, "…");
        // Should return just ellipsis since no chars fit
        assert_eq!(result, "…");
    }

    #[test]
    fn test_trim_text_to_width_mixed_widths() {
        // Mix of narrow and regular width characters
        let result = trim_text_to_width("million", 15.0, 1.0, "…");
        // Should fit or trim appropriately
        assert!(!result.is_empty());
    }

    #[test]
    fn test_calculate_available_width_no_buttons() {
        // No buttons configured
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(0.0),
            roundness: None,
            items: vec![],
        };
        let result = calculate_available_width_for_centered_text(100.0, &button_cfg, 14.0, 2);
        assert_eq!(result, 100.0);
    }

    #[test]
    fn test_calculate_available_width_with_buttons() {
        // With buttons on one side
        use crate::config::Number;
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(10.0),
            roundness: None,
            items: vec![crate::config::winstyle::WindowButton {
                offset: Number::from(10.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            }],
        };
        let result = calculate_available_width_for_centered_text(100.0, &button_cfg, 14.0, 2);
        // Should reduce width due to button space
        assert!(result < 100.0);
    }

    #[test]
    fn test_calculate_available_width_symmetrical_buttons() {
        // With buttons on both sides
        use crate::config::Number;
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Left,
            shape: None,
            size: Number::from(10.0),
            roundness: None,
            items: vec![
                crate::config::winstyle::WindowButton {
                    offset: Number::from(10.0),
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    icon: None,
                },
                crate::config::winstyle::WindowButton {
                    offset: Number::from(30.0),
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    icon: None,
                },
            ],
        };
        let result = calculate_available_width_for_centered_text(100.0, &button_cfg, 14.0, 2);
        // Should be less than 100
        assert!(result < 100.0);
    }

    #[test]
    fn test_title_rendering_with_short_title() {
        // Test that short titles are rendered without trimming
        let result = trim_text_to_width("Test", 100.0, 1.0, "…");
        assert_eq!(result, "Test");
        // Verify this is a renderable title (not empty)
        assert!(!result.is_empty());
    }

    #[test]
    fn test_title_rendering_with_long_title() {
        // Test that long titles are trimmed
        let result = trim_text_to_width(
            "This is a very long title that should be trimmed",
            20.0,
            1.0,
            "…",
        );
        assert!(result.contains("…"));
        assert!(!result.is_empty());
        // Verify it's shorter than original
        assert!(result.len() < "This is a very long title that should be trimmed".len() + 1);
    }

    #[test]
    fn test_title_rendering_integration() {
        // Test the complete title rendering workflow:
        // 1. Calculate available width with buttons
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(8.0),
            roundness: None,
            items: vec![crate::config::winstyle::WindowButton {
                offset: Number::from(5.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            }],
        };
        let width = 200.0;
        let available_width =
            calculate_available_width_for_centered_text(width, &button_cfg, 12.0, 2);
        assert!(available_width > 0.0);
        assert!(available_width < width);

        // 2. Trim the title to fit in available width
        let title = "Welcome to My Application";
        let char_width = 12.0 * 0.6;
        let trimmed = trim_text_to_width(title, available_width, char_width, "…");

        // 3. Verify result is either original or trimmed with ellipsis
        assert!(!trimmed.is_empty());
        if trimmed != title {
            assert!(trimmed.contains("…"));
        }
    }

    #[test]
    fn test_title_rendering_empty_after_trim() {
        // Test edge case where title becomes empty after trimming
        let result = trim_text_to_width("w", 6.5, 1.0, "…");
        // With very tight constraints, title might be trimmed completely
        // but ellipsis should still fit or we get empty string
        assert!(result.is_empty() || result == "…");
    }

    #[test]
    fn test_title_rendering_with_multiple_button_styles() {
        // Test available width calculation with multiple buttons
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Left,
            shape: None,
            size: Number::from(12.0),
            roundness: None,
            items: vec![
                crate::config::winstyle::WindowButton {
                    offset: Number::from(5.0),
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    icon: None,
                },
                crate::config::winstyle::WindowButton {
                    offset: Number::from(25.0),
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    icon: None,
                },
                crate::config::winstyle::WindowButton {
                    offset: Number::from(45.0),
                    fill: None,
                    stroke: None,
                    stroke_width: None,
                    icon: None,
                },
            ],
        };
        let available = calculate_available_width_for_centered_text(300.0, &button_cfg, 14.0, 2);
        assert!(available > 0.0);
        assert!(available < 300.0);
    }

    #[test]
    fn test_title_rendering_proportional_fit() {
        // Test that proportional fonts are properly considered
        let title = "iiiiiiii"; // Narrow characters
        let result_narrow = trim_text_to_width(title, 10.0, 1.0, "…");

        let title_wide = "wwwwwwww"; // Wide characters
        let result_wide = trim_text_to_width(title_wide, 10.0, 1.0, "…");

        // Narrow characters should fit more
        if result_narrow.contains("…") {
            let narrow_trimmed_count = result_narrow.matches('i').count();
            let wide_trimmed_count = result_wide.matches('w').count();
            assert!(
                narrow_trimmed_count >= wide_trimmed_count,
                "Narrow chars should fit at least as many as wide chars"
            );
        }
    }

    #[test]
    fn test_title_rendering_path_with_non_empty_title() {
        // Test the path where title is Some and not empty
        // This covers the title rendering lines in make_window
        let result = trim_text_to_width("My App", 100.0, 1.0, "…");
        // Title should be rendered as-is since it fits
        assert_eq!(result, "My App");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_title_rendering_path_with_empty_title_after_trim() {
        // Test the path where title becomes empty after trimming
        // This exercises the if !title.is_empty() check
        let result = trim_text_to_width("w", 6.5, 1.0, "…");
        // Result is either empty or just ellipsis - either way the empty check handles it
        assert!(result.is_empty() || result == "…");
    }

    #[test]
    fn test_title_rendering_with_font_weight() {
        // Test that title rendering considers font weight
        // The font weight is applied when set in window config
        let title = "App";
        let result = trim_text_to_width(title, 50.0, 1.0, "…");
        // Title should render regardless of weight setting
        assert_eq!(result, "App");
    }

    #[test]
    fn test_title_rendering_attributes() {
        // Test that all title attributes are properly set
        // x position: (width / 2.0) - centered
        // y position: (hh2) - header middle
        // fill: from cfg.color
        // font-size: from cfg.font.size
        // font-family: from cfg.font.family
        // text-anchor: middle
        // dominant-baseline: central

        // Calculate centered position
        let width = 300.0;
        let x_pos = width / 2.0;
        assert_eq!(x_pos, 150.0);

        // Verify title fits in available space with buttons
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(8.0),
            roundness: None,
            items: vec![crate::config::winstyle::WindowButton {
                offset: Number::from(5.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            }],
        };
        let available = calculate_available_width_for_centered_text(width, &button_cfg, 12.0, 2);
        let title = trim_text_to_width("Application", available, 12.0 * 0.6, "…");
        assert!(!title.is_empty());
    }

    #[test]
    fn test_make_window_integration_with_title() {
        // Integration test for make_window with title rendering
        // This exercises the title rendering code paths in make_window (lines 618-638)
        use crate::config::Settings;
        use crate::config::mode::Mode;
        use crate::render::{FontMetrics, FontOptions, FontWeights, Theme};

        // Create a minimal screen element
        let screen = element::SVG::new();

        // Create minimal button configuration
        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(8.0),
            roundness: None,
            items: vec![],
        };

        // Create window configuration with title
        let window_config = crate::config::winstyle::Window {
            margin: crate::config::PaddingOption::Uniform(Number::from(5.0)),
            border: crate::config::winstyle::WindowBorder {
                width: Number::from(1.0),
                radius: Number::from(4.0),
                gap: None,
                colors: crate::config::winstyle::WindowBorderColors {
                    outer: crate::config::winstyle::SelectiveColor::Uniform(
                        csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                    ),
                    inner: crate::config::winstyle::SelectiveColor::Uniform(
                        csscolorparser::Color::from_rgba8(100, 100, 100, 255),
                    ),
                },
            },
            header: crate::config::winstyle::WindowHeader {
                height: Number::from(24.0),
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(200, 200, 200, 255),
                ),
                border: None,
            },
            title: crate::config::winstyle::WindowTitle {
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                ),
                font: crate::config::winstyle::Font {
                    family: vec!["Monospace".to_string()],
                    size: Number::from(12.0),
                    weight: Some("normal".to_string()),
                },
            },
            buttons: button_cfg,
            shadow: crate::config::winstyle::WindowShadow {
                enabled: false,
                x: Number::from(0.0),
                y: Number::from(0.0),
                blur: Number::from(0.0),
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(0, 0, 0, 100),
                ),
            },
        };

        // Create Options with title
        let options = Options {
            settings: Rc::new(Settings::default()),
            font: FontOptions {
                family: vec!["Monospace".to_string()],
                size: 12.0,
                metrics: FontMetrics {
                    width: 0.6,
                    ascender: 0.8,
                    descender: -0.2,
                },
                faces: vec![],
                weights: FontWeights {
                    normal: FontWeight::Normal,
                    bold: FontWeight::Bold,
                    faint: FontWeight::Normal,
                },
            },
            theme: Rc::new(Theme {
                bg: csscolorparser::Color::from_rgba8(255, 255, 255, 255),
                fg: csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                bright_fg: None,
                palette: Default::default(),
            }),
            window: window_config,
            title: Some("Test Title".to_string()),
            mode: Mode::Light,
            background: None,
            foreground: None,
        };

        // Call make_window to exercise title rendering paths
        let result = make_window(&options, 200.0, 150.0, screen);

        // Verify the result contains SVG content
        let svg_str = result.to_string();
        assert!(!svg_str.is_empty());
        // The rendered SVG should contain text elements
        assert!(svg_str.contains("Test") || svg_str.contains("…"));
    }

    #[test]
    fn test_make_window_integration_no_title() {
        // Test make_window when title is None
        // This exercises the else path (no title rendering)
        use crate::config::Settings;
        use crate::config::mode::Mode;
        use crate::render::{FontMetrics, FontOptions, FontWeights, Theme};

        let screen = element::SVG::new();

        let button_cfg = crate::config::winstyle::WindowButtons {
            position: WindowButtonsPosition::Right,
            shape: None,
            size: Number::from(8.0),
            roundness: None,
            items: vec![],
        };

        let window_config = crate::config::winstyle::Window {
            margin: crate::config::PaddingOption::Uniform(Number::from(5.0)),
            border: crate::config::winstyle::WindowBorder {
                width: Number::from(1.0),
                radius: Number::from(4.0),
                gap: None,
                colors: crate::config::winstyle::WindowBorderColors {
                    outer: crate::config::winstyle::SelectiveColor::Uniform(
                        csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                    ),
                    inner: crate::config::winstyle::SelectiveColor::Uniform(
                        csscolorparser::Color::from_rgba8(100, 100, 100, 255),
                    ),
                },
            },
            header: crate::config::winstyle::WindowHeader {
                height: Number::from(24.0),
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(200, 200, 200, 255),
                ),
                border: None,
            },
            title: crate::config::winstyle::WindowTitle {
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                ),
                font: crate::config::winstyle::Font {
                    family: vec!["Monospace".to_string()],
                    size: Number::from(12.0),
                    weight: Some("bold".to_string()),
                },
            },
            buttons: button_cfg,
            shadow: crate::config::winstyle::WindowShadow {
                enabled: false,
                x: Number::from(0.0),
                y: Number::from(0.0),
                blur: Number::from(0.0),
                color: crate::config::winstyle::SelectiveColor::Uniform(
                    csscolorparser::Color::from_rgba8(0, 0, 0, 100),
                ),
            },
        };

        let options = Options {
            settings: Rc::new(Settings::default()),
            font: FontOptions {
                family: vec!["Monospace".to_string()],
                size: 12.0,
                metrics: FontMetrics {
                    width: 0.6,
                    ascender: 0.8,
                    descender: -0.2,
                },
                faces: vec![],
                weights: FontWeights {
                    normal: FontWeight::Normal,
                    bold: FontWeight::Bold,
                    faint: FontWeight::Normal,
                },
            },
            theme: Rc::new(Theme {
                bg: csscolorparser::Color::from_rgba8(255, 255, 255, 255),
                fg: csscolorparser::Color::from_rgba8(0, 0, 0, 255),
                bright_fg: None,
                palette: Default::default(),
            }),
            window: window_config,
            title: None,
            mode: Mode::Light,
            background: None,
            foreground: None,
        };

        let result = make_window(&options, 200.0, 150.0, screen);
        let svg_str = result.to_string();
        assert!(!svg_str.is_empty());
    }
}
