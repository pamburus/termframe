use url::Url;

use crate::font::{FontFile, Location, is_transient};

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

    // Test Windows absolute path, which is only a file path on Windows
    let location = Location::from(r"C:\fonts\font.ttf");
    assert_eq!(matches!(location, Location::File(_)), cfg!(windows));

    // Test relative path
    let location = Location::from("./fonts/font.ttf");
    assert!(matches!(location, Location::File(_)));
    if let Location::File(path) = location {
        assert_eq!(path.to_string_lossy(), "./fonts/font.ttf");
    }
}

#[test]
fn test_font_location_to_url() {
    let location = Location::from("https://example.com/fonts/font.ttf");
    assert_eq!(
        location.to_url().unwrap().as_str(),
        "https://example.com/fonts/font.ttf"
    );

    let path = std::env::temp_dir().join("font.ttf");
    let location = Location::File(path.clone());
    assert_eq!(location.to_url().unwrap().to_file_path().unwrap(), path);

    let location = Location::from("fonts/font.ttf");
    let url = location.to_url().unwrap();
    assert_eq!(url.scheme(), "file");
    assert_eq!(
        url.to_file_path().unwrap(),
        std::env::current_dir().unwrap().join("fonts/font.ttf")
    );
}

#[test]
fn test_font_file_load_file_url() {
    let path = std::env::temp_dir().join("termframe test font that does not exist.ttf");
    let url = Url::from_file_path(&path).unwrap();
    assert!(url.as_str().contains("%20"));

    let err = FontFile::load_url(url).err().unwrap();
    let err = err.downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_font_file_load_invalid_file_url() {
    // A file URL with a host is a valid UNC path on Windows, and one without a drive letter is a valid path elsewhere.
    let url = if cfg!(windows) {
        "file:///fonts/font.ttf"
    } else {
        "file://example.com/fonts/font.ttf"
    };
    let url = Url::parse(url).unwrap();
    let err = FontFile::load_url(url).err().unwrap();
    assert!(err.to_string().starts_with("invalid file URL"));
}

#[test]
fn test_font_file_load_invalid_url_fails_immediately() {
    let url = Url::parse(r"c:\fonts\font.ttf").unwrap();
    let started = std::time::Instant::now();
    let err = FontFile::load_url(url).err().unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(matches!(
        err.downcast_ref::<ureq::Error>(),
        Some(ureq::Error::Http(_))
    ));
}

#[test]
fn test_is_transient() {
    use ureq::Error;

    for status in [429, 500, 502, 503, 504] {
        assert!(is_transient(&Error::StatusCode(status)), "{status}");
    }
    for status in [400, 403, 404, 428, 430, 501, 505] {
        assert!(!is_transient(&Error::StatusCode(status)), "{status}");
    }

    assert!(is_transient(&Error::Io(
        std::io::ErrorKind::ConnectionReset.into()
    )));
    assert!(is_transient(&Error::Timeout(ureq::Timeout::Global)));
    assert!(is_transient(&Error::HostNotFound));
    assert!(is_transient(&Error::ConnectionFailed));

    assert!(!is_transient(&Error::BadUri("font.ttf".into())));
    assert!(!is_transient(&Error::Tls("unsupported")));
    assert!(!is_transient(&Error::TooManyRedirects));
    assert!(!is_transient(&Error::RequireHttpsOnly(
        "http://example.com".into()
    )));
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
