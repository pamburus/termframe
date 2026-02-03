use csscolorparser::Color;
use std::rc::Rc;
use termwiz::color::SrgbaTuple;

use crate::render::{
    CharSet, FontFace, FontMetrics, FontOptions, FontStyle, FontWeight, FontWeights,
};

// Define a mock CharSet for testing since we can't access the internal one
#[derive(Debug)]
struct MockCharSet;

impl MockCharSet {
    fn new(_f: impl Fn(char) -> bool) -> Self {
        Self
    }
}

impl CharSet for MockCharSet {
    fn has_char(&self, _ch: char) -> bool {
        true
    }
}

#[test]
fn test_font_options() {
    // Create font options for testing
    let options = FontOptions {
        family: vec!["Monospace".to_string(), "Consolas".to_string()],
        size: 14.0,
        metrics: FontMetrics {
            width: 0.6,
            ascender: 0.8,
            descender: -0.2,
        },
        faces: vec![
            FontFace {
                family: "Monospace".to_string(),
                weight: FontWeight::Normal,
                style: Some(FontStyle::Normal),
                format: None,
                url: "monospace.ttf".to_string(),
                chars: Rc::new(MockCharSet::new(|_| true)),
                metrics_match: true,
            },
            FontFace {
                family: "Consolas".to_string(),
                weight: FontWeight::Bold,
                style: Some(FontStyle::Italic),
                format: None,
                url: "consolas.ttf".to_string(),
                chars: Rc::new(MockCharSet::new(|_| true)),
                metrics_match: true,
            },
        ],
        weights: FontWeights {
            normal: FontWeight::Normal,
            bold: FontWeight::Bold,
            faint: FontWeight::Fixed(300),
        },
    };

    // Verify font options properties
    assert_eq!(options.family.len(), 2);
    assert_eq!(options.family[0], "Monospace");
    assert_eq!(options.family[1], "Consolas");
    assert_eq!(options.size, 14.0);
    assert_eq!(options.metrics.width, 0.6);
    assert_eq!(options.metrics.ascender, 0.8);
    assert_eq!(options.metrics.descender, -0.2);

    // Verify font faces
    assert_eq!(options.faces.len(), 2);

    // First face
    assert_eq!(options.faces[0].family, "Monospace");
    assert!(matches!(options.faces[0].weight, FontWeight::Normal));
    assert!(matches!(options.faces[0].style, Some(FontStyle::Normal)));

    // Second face
    assert_eq!(options.faces[1].family, "Consolas");
    assert!(matches!(options.faces[1].weight, FontWeight::Bold));
    assert!(matches!(options.faces[1].style, Some(FontStyle::Italic)));

    // Verify font weights
    assert!(matches!(options.weights.normal, FontWeight::Normal));
    assert!(matches!(options.weights.bold, FontWeight::Bold));
    assert!(matches!(options.weights.faint, FontWeight::Fixed(300)));
}

#[test]
fn test_font_weight_range() {
    // Test FontWeight range method
    assert_eq!(FontWeight::Normal.range(), (400, 400));
    assert_eq!(FontWeight::Bold.range(), (600, 600));
    assert_eq!(FontWeight::Fixed(400).range(), (400, 400));
    assert_eq!(FontWeight::Variable(300, 700).range(), (300, 700));
}

#[test]
fn test_color_conversion() {
    // Test Color to SrgbaTuple conversion
    let color = Color::from_rgba8(255, 0, 0, 255); // Red
    let rgba: SrgbaTuple = crate::Convert::convert(&color);
    assert_eq!(rgba.as_rgba_u8(), (255, 0, 0, 255));

    // Test SrgbaTuple to Color conversion
    let rgba = SrgbaTuple::from((0, 255, 0, 255)); // Green
    let color: Color = crate::Convert::convert(&rgba);
    let rgba8 = color.to_rgba8();
    assert_eq!((rgba8[0], rgba8[1], rgba8[2], rgba8[3]), (0, 255, 0, 255));
}
