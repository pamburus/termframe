// std imports
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::LazyLock,
};

// third-party imports
use csscolorparser::Color;
use termwiz::color::ColorAttribute;

// local imports
use crate::config::{
    self,
    {mode::Mode, theme::ThemeConfig},
};

// ---

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AdaptiveTheme {
    pub light: Rc<Theme>,
    pub dark: Rc<Theme>,
}

impl AdaptiveTheme {
    #[allow(dead_code)]
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        match cfg {
            ThemeConfig::Fixed(cfg) => {
                let theme = Rc::new(Theme::from_config(&cfg.colors));
                Self {
                    light: theme.clone(),
                    dark: theme,
                }
            }
            ThemeConfig::Adaptive(cfg) => {
                let light = Rc::new(Theme::from_config(&cfg.modes.light));
                let dark = Rc::new(Theme::from_config(&cfg.modes.dark));
                Self { light, dark }
            }
        }
    }

    #[allow(dead_code)]
    pub fn resolve(self, mode: Mode) -> Rc<Theme> {
        match mode {
            Mode::Light => self.light,
            Mode::Dark => self.dark,
        }
    }
}

impl Default for AdaptiveTheme {
    fn default() -> Self {
        let bg = Color::from_rgba8(0x28, 0x2c, 0x30, 0xff);
        let fg = Color::from_rgba8(0xac, 0xb2, 0xbe, 0xff);
        let mut palette = Palette::default();
        palette[0] = Color::from_rgba8(0x28, 0x2c, 0x34, 0xff); // black
        palette[1] = Color::from_rgba8(0xd1, 0x72, 0x77, 0xff); // red
        palette[2] = Color::from_rgba8(0xa1, 0xc2, 0x81, 0xff); // green
        palette[3] = Color::from_rgba8(0xde, 0x9b, 0x64, 0xff); // yellow
        palette[4] = Color::from_rgba8(0x74, 0xad, 0xe9, 0xff); // blue
        palette[5] = Color::from_rgba8(0xbb, 0x7c, 0xd7, 0xff); // magenta
        palette[6] = Color::from_rgba8(0x29, 0xa9, 0xbc, 0xff); // cyan
        palette[7] = Color::from_rgba8(0xac, 0xb2, 0xbe, 0xff); // white
        palette[8] = Color::from_rgba8(0x67, 0x6f, 0x82, 0xff); // bright black
        palette[9] = Color::from_rgba8(0xe6, 0x67, 0x6d, 0xff); // bright red
        palette[10] = Color::from_rgba8(0xa9, 0xd4, 0x7f, 0xff); // bright green
        palette[11] = Color::from_rgba8(0xde, 0x9b, 0x64, 0xff); // bright yellow
        palette[12] = Color::from_rgba8(0x66, 0xac, 0xff, 0xff); // bright blue
        palette[13] = Color::from_rgba8(0xc6, 0x71, 0xeb, 0xff); // bright magenta
        palette[14] = Color::from_rgba8(0x69, 0xc6, 0xd1, 0xff); // bright cyan
        palette[15] = Color::from_rgba8(0xcc, 0xcc, 0xcc, 0xff); // bright white
        let dark = Theme { bg, fg, palette }.into();

        let bg = Color::from_rgba8(0xf9, 0xf9, 0xf9, 0xff);
        let fg = Color::from_rgba8(0x2a, 0x2c, 0x33, 0xff);
        let mut palette = Palette::default();
        palette[0] = Color::from_rgba8(0x00, 0x00, 0x00, 0xff); // black
        palette[1] = Color::from_rgba8(0xc9, 0x1b, 0x00, 0xff); // red
        palette[2] = Color::from_rgba8(0x00, 0xc2, 0x00, 0xff); // green
        palette[3] = Color::from_rgba8(0xc7, 0xc4, 0x00, 0xff); // yellow
        palette[4] = Color::from_rgba8(0x02, 0x25, 0xc7, 0xff); // blue
        palette[5] = Color::from_rgba8(0xc9, 0x30, 0xc7, 0xff); // magenta
        palette[6] = Color::from_rgba8(0x00, 0xc5, 0xc7, 0xff); // cyan
        palette[7] = Color::from_rgba8(0xc7, 0xc7, 0xc7, 0xff); // white
        palette[8] = Color::from_rgba8(0x67, 0x67, 0x67, 0xff); // bright black
        palette[9] = Color::from_rgba8(0xff, 0x6d, 0x67, 0xff); // bright red
        palette[10] = Color::from_rgba8(0x5f, 0xf9, 0x67, 0xff); // bright green
        palette[11] = Color::from_rgba8(0xfe, 0xfb, 0x67, 0xff); // bright yellow
        palette[12] = Color::from_rgba8(0x68, 0x71, 0xff, 0xff); // bright blue
        palette[13] = Color::from_rgba8(0xff, 0x76, 0xff, 0xff); // bright magenta
        palette[14] = Color::from_rgba8(0x5f, 0xfd, 0xff, 0xff); // bright cyan
        palette[15] = Color::from_rgba8(0xff, 0xfe, 0xff, 0xff); // bright white
        let light = Theme { bg, fg, palette }.into();

        Self { dark, light }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub palette: Palette,
}

impl Theme {
    pub fn from_config(cfg: &config::theme::Colors) -> Self {
        let bg = cfg.background.clone();
        let fg = cfg.foreground.clone();
        let palette = Palette::from_config(&cfg.palette);
        Self { bg, fg, palette }
    }

    pub fn resolve(&self, attr: ColorAttribute) -> Option<Color> {
        match attr {
            ColorAttribute::Default => None,
            ColorAttribute::PaletteIndex(idx) => Some(self.palette[idx as usize].clone()),
            ColorAttribute::TrueColorWithDefaultFallback(c)
            | ColorAttribute::TrueColorWithPaletteFallback(c, _) => {
                Some(Color::new(c.0.into(), c.1.into(), c.2.into(), c.3.into()))
            }
        }
    }
}

// ---

#[derive(Debug, Clone)]
pub struct Palette([Color; 256]);

impl Deref for Palette {
    type Target = [Color; 256];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Palette {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Palette {
    pub fn new(colors: [Color; 256]) -> Self {
        Self(colors)
    }

    pub fn from_config(cfg: &config::theme::Palette) -> Self {
        let mut colors = Self::make_default().0;
        for (i, c) in cfg.iter() {
            colors[*i as usize] = c.clone();
        }
        Self::new(colors)
    }

    fn make_default() -> Self {
        let colors = std::array::from_fn(|i| {
            let i = i as u8;
            match i {
                0 => Color::from_rgba8(0x00, 0x00, 0x00, 0xff),
                7 => Color::from_rgba8(0xc0, 0xc0, 0xc0, 0xff),
                8 => Color::from_rgba8(0x80, 0x80, 0x80, 0xff),
                15 => Color::from_rgba8(0xff, 0xff, 0xff, 0xff),
                0..16 => {
                    let k = if i & 8 != 0 { 0xff } else { 0x80 };
                    let r = (i & 1) * k;
                    let g = ((i >> 1) & 1) * k;
                    let b = ((i >> 2) & 1) * k;
                    Color::from_rgba8(r, g, b, 0xff)
                }
                16..232 => {
                    let i = i - 16;
                    let c: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
                    let r = c[((i / 36) % 6) as usize];
                    let g = c[((i / 6) % 6) as usize];
                    let b = c[(i % 6) as usize];
                    Color::from_rgba8(r, g, b, 0xff)
                }
                232..=255 => {
                    let i = i - 232;
                    let c = 8 + i * 10;
                    Color::from_rgba8(c, c, c, 0xff)
                }
            }
        });
        Self::new(colors)
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::make_default()
    }
}

impl Default for &Palette {
    fn default() -> Self {
        static DEFAULT: LazyLock<Palette> = LazyLock::new(|| Palette::make_default());
        &DEFAULT
    }
}
