use std::{rc::Rc, sync::Arc};

use allsorts::woff2;
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

        let mut fontdb = fontdb::Database::new();
        fontdb.load_system_fonts();
        // for font in &self.options.font.faces {
        //     let mut response = ureq::get(&font.url).call()?;
        //     fontdb.load_font_data(response.body_mut().read_to_vec()?);
        // }

        let scale = 4.0;
        let opt = usvg::Options {
            dpi: 192.0,
            fontdb: Arc::new(fontdb),
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
