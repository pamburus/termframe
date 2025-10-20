use crate::fontformat::FontFormat;

#[test]
fn test_font_format() {
    // Test mime types
    assert_eq!(FontFormat::Ttf.mime(), "font/ttf");
    assert_eq!(FontFormat::Otf.mime(), "font/otf");
    assert_eq!(FontFormat::Woff.mime(), "font/woff");
    assert_eq!(FontFormat::Woff2.mime(), "font/woff2");
}
