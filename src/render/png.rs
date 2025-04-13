use std::{rc::Rc, sync::Arc};

use fontdb::{Language, Source, Style};
use termwiz::surface::Surface;
use tiny_skia::{Pixmap, Transform};

use super::svg::SvgRenderer;

pub use super::{Options, Render, Result};

/// A renderer for generating PNG representations of terminal surfaces.
pub struct PngRenderer {
    svg: SvgRenderer,
    options: Rc<Options>,
}

impl PngRenderer {
    /// Creates a new `SvgRenderer` with the given options.
    pub fn new(options: Rc<Options>) -> Self {
        Self {
            svg: SvgRenderer::new(options.clone()),
            options,
        }
    }

    /// Renders the given terminal surface to the specified target as an SVG.
    pub fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()> {
        let mut buf = Vec::new();

        self.svg.render(surface, &mut buf)?;

        let mut fonts = fontdb::Database::new();
        for (i, font) in self.options.font.faces.iter().enumerate() {
            let mut response = ureq::get(&font.url).call()?;
            let mut font_data = response.body_mut().read_to_vec()?;
            if font.url.ends_with(".woff2") {
                font_data = woff2::convert_woff2_to_ttf(&mut font_data.as_slice())?;
            }
            let source = Source::Binary(Arc::new(font_data));
            let ids = fonts.load_font_source(source.clone());

            if ids.len() == 0 {
                log::warn!("failed to load font {}", &font.family);
                continue;
            }
            if ids.len() > 1 {
                log::warn!(
                    "multiple fonts found ({}) in a single file {}",
                    ids.len(),
                    &font.url,
                );
            }

            for weight in (font.weight.range().0..=font.weight.range().1).step_by(100) {
                log::debug!(
                    "add font face info #{i:02} family={:?} weight={weight} style={:?} file={}",
                    &font.family,
                    &font.style,
                    &font.url,
                );
                fonts.push_face_info(fontdb::FaceInfo {
                    id: ids[0],
                    source: source.clone(),
                    index: 0,
                    families: vec![(font.family.clone(), Language::English_UnitedStates)],
                    post_script_name: font.postscript_name.clone(),
                    style: match font.style {
                        None | Some(super::FontStyle::Normal) => Style::Normal,
                        Some(super::FontStyle::Italic) => Style::Italic,
                        Some(super::FontStyle::Oblique) => Style::Oblique,
                    },
                    weight: fontdb::Weight(weight),
                    stretch: fontdb::Stretch::Normal,
                    monospaced: font.monospaced,
                });
            }
        }

        let scale = 4.0;
        let opt = usvg::Options {
            dpi: 192.0,
            fontdb: Arc::new(fonts),
            ..Default::default()
        };
        let rtree = usvg::Tree::from_data(&buf, &opt)?;

        let size = {
            let size = rtree.size();
            (
                (size.width() * scale) as u32,
                (size.height() * scale) as u32,
            )
        };

        let mut pixmap = Pixmap::new(size.0, size.1).unwrap();
        resvg::render(
            &rtree,
            Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );
        let buf = pixmap.encode_png()?;

        target.write(&buf)?;

        Ok(())
    }
}

impl Render for PngRenderer {
    fn render(&self, surface: &Surface, target: &mut dyn std::io::Write) -> Result<()> {
        Self::render(self, surface, target)
    }
}
