use crate::config::mode::Mode;
use crate::theme::AdaptiveTheme;

#[test]
fn test_adaptive_theme() {
    // Test default adaptive theme
    let adaptive = AdaptiveTheme::default();
    let light_theme = adaptive.clone().resolve(Mode::Light);
    let dark_theme = adaptive.resolve(Mode::Dark);

    // Just verify we get different themes by checking they're not the same string
    // We can't directly compare the themes since Theme doesn't implement PartialEq
    // Instead, verify they have different string representations
    assert!(format!("{:?}", light_theme) != format!("{:?}", dark_theme));
}

#[test]
fn test_palette_from_config_skips_unresolved_indices() {
    use crate::config::theme::{Palette as PaletteConfig, PaletteIndex};
    use crate::theme::Palette;
    use csscolorparser::Color;

    let red = Color::from_rgba8(0xff, 0, 0, 0xff);
    let cfg = PaletteConfig::from([
        (PaletteIndex::U8(1), red),
        (PaletteIndex::String("2".into()), red),
        (PaletteIndex::String("red".into()), red),
    ]);

    let default = Palette::from_config(&PaletteConfig::new());
    let palette = Palette::from_config(&cfg);
    assert_eq!(palette[1], red);
    assert_eq!(palette[2], red);
    assert_eq!(palette[0], default[0]);
    assert_eq!(palette[3..], default[3..]);
}
