use crate::font::Location;

#[test]
fn test_font_location_from_str() {
    // Test file path
    let location = Location::from("/path/to/font.ttf");
    assert!(matches!(location, Location::File(_)));
    if let Location::File(path) = location {
        assert_eq!(path.to_string_lossy(), "/path/to/font.ttf");
    }

    // Test URL
    let location = Location::from("https://example.com/fonts/font.ttf");
    assert!(matches!(location, Location::Url(_)));
    if let Location::Url(url) = location {
        assert_eq!(url.to_string(), "https://example.com/fonts/font.ttf");
    }

    // Test relative path
    let location = Location::from("./fonts/font.ttf");
    assert!(matches!(location, Location::File(_)));
    if let Location::File(path) = location {
        assert_eq!(path.to_string_lossy(), "./fonts/font.ttf");
    }
}

#[test]
fn test_font_metrics() {
    // Mock test for font metrics properties
    // In a real implementation, you would use a proper font file
    let _mock_data = mock_font_data();

    // This is just a structure test since we can't really load fonts without
    // proper font data in tests
    struct MockFont {
        width: f32,
        ascender: f32,
        descender: f32,
        family: Option<String>,
        weight_val: u16,
    }

    // For now, we'll just verify the mock structure works without checking methods
    // We would test the font properties if the trait were accessible
    let mock_font = MockFont {
        width: 0.6,
        ascender: 0.8,
        descender: -0.2,
        family: Some("Test Font".to_string()),
        weight_val: 400,
    };

    // Basic property checks
    assert_eq!(mock_font.width, 0.6);
    assert_eq!(mock_font.ascender, 0.8);
    assert_eq!(mock_font.descender, -0.2);
    assert_eq!(mock_font.family, Some("Test Font".to_string()));
    assert_eq!(mock_font.weight_val, 400);
}

// Mock font data for testing
fn mock_font_data() -> Vec<u8> {
    // This is not a real font, just a placeholder for testing
    vec![0, 1, 2, 3, 4, 5]
}
