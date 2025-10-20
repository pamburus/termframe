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
