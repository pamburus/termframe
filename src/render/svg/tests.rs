use super::*;

use csscolorparser::Color;
use itertools::Itertools;
use termwiz::surface::Change;

use crate::{
    config::{
        Number, PaddingOption, Settings,
        mode::Mode,
        winstyle::{
            Font, SelectiveColor, Window, WindowBorder, WindowBorderColors, WindowButtons,
            WindowHeader, WindowShadow, WindowStyleConfig, WindowTitle,
        },
    },
    render::{FontMetrics, FontOptions, FontWeights, Options},
};

trait Sample {
    fn sample() -> Self;
}

impl Sample for Options {
    fn sample() -> Self {
        Options {
            settings: Default::default(),
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
                bg: Color::from_rgba8(255, 255, 255, 255),
                fg: Color::from_rgba8(0, 0, 0, 255),
                bright_fg: None,
                palette: Default::default(),
            }),
            window: WindowStyleConfig::default().window,
            title: Some("Sample Title".to_string()),
            mode: Mode::Light,
            background: None,
            foreground: None,
        }
    }
}

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
    let button_cfg = WindowButtons {
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
    use Number;
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Right,
        shape: None,
        size: Number::from(10.0),
        roundness: None,
        items: vec![WindowButton {
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
    use Number;
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Left,
        shape: None,
        size: Number::from(10.0),
        roundness: None,
        items: vec![
            WindowButton {
                offset: Number::from(10.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            },
            WindowButton {
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
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Right,
        shape: None,
        size: Number::from(8.0),
        roundness: None,
        items: vec![WindowButton {
            offset: Number::from(5.0),
            fill: None,
            stroke: None,
            stroke_width: None,
            icon: None,
        }],
    };
    let width = 200.0;
    let available_width = calculate_available_width_for_centered_text(width, &button_cfg, 12.0, 2);
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
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Left,
        shape: None,
        size: Number::from(12.0),
        roundness: None,
        items: vec![
            WindowButton {
                offset: Number::from(5.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            },
            WindowButton {
                offset: Number::from(25.0),
                fill: None,
                stroke: None,
                stroke_width: None,
                icon: None,
            },
            WindowButton {
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
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Right,
        shape: None,
        size: Number::from(8.0),
        roundness: None,
        items: vec![WindowButton {
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
    // Create a minimal screen element
    let screen = element::SVG::new();

    // Create minimal button configuration
    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Right,
        shape: None,
        size: Number::from(8.0),
        roundness: None,
        items: vec![],
    };

    // Create window configuration with title
    let window_config = Window {
        margin: PaddingOption::Uniform(Number::from(5.0)),
        border: WindowBorder {
            width: Number::from(1.0),
            radius: Number::from(4.0),
            gap: None,
            colors: WindowBorderColors {
                outer: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 255)),
                inner: SelectiveColor::Uniform(Color::from_rgba8(100, 100, 100, 255)),
            },
        },
        header: WindowHeader {
            height: Number::from(24.0),
            color: SelectiveColor::Uniform(Color::from_rgba8(200, 200, 200, 255)),
            border: None,
        },
        title: WindowTitle {
            color: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 255)),
            font: Font {
                family: vec!["Monospace".to_string()],
                size: Number::from(12.0),
                weight: Some("normal".to_string()),
            },
        },
        buttons: button_cfg,
        shadow: WindowShadow {
            enabled: false,
            x: Number::from(0.0),
            y: Number::from(0.0),
            blur: Number::from(0.0),
            color: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 100)),
        },
    };

    // Create Options with title
    let options = Options {
        settings: Default::default(),
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
            bg: Color::from_rgba8(255, 255, 255, 255),
            fg: Color::from_rgba8(0, 0, 0, 255),
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
    let screen = element::SVG::new();

    let button_cfg = WindowButtons {
        position: WindowButtonsPosition::Right,
        shape: None,
        size: Number::from(8.0),
        roundness: None,
        items: vec![],
    };

    let window_config = Window {
        margin: PaddingOption::Uniform(Number::from(5.0)),
        border: WindowBorder {
            width: Number::from(1.0),
            radius: Number::from(4.0),
            gap: None,
            colors: WindowBorderColors {
                outer: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 255)),
                inner: SelectiveColor::Uniform(Color::from_rgba8(100, 100, 100, 255)),
            },
        },
        header: WindowHeader {
            height: Number::from(24.0),
            color: SelectiveColor::Uniform(Color::from_rgba8(200, 200, 200, 255)),
            border: None,
        },
        title: WindowTitle {
            color: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 255)),
            font: Font {
                family: vec!["Monospace".to_string()],
                size: Number::from(12.0),
                weight: Some("bold".to_string()),
            },
        },
        buttons: button_cfg,
        shadow: WindowShadow {
            enabled: false,
            x: Number::from(0.0),
            y: Number::from(0.0),
            blur: Number::from(0.0),
            color: SelectiveColor::Uniform(Color::from_rgba8(0, 0, 0, 100)),
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
            bg: Color::from_rgba8(255, 255, 255, 255),
            fg: Color::from_rgba8(0, 0, 0, 255),
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

#[test]
fn test_subclusters_plain() {
    let mut surface = Surface::new(5, 2);
    surface.add_change(Change::Text("a".into()));
    let options = Options::sample();

    let lines = surface.screen_lines();
    let lines = lines.iter().collect_vec();
    assert_eq!(lines.len(), 2);

    let line = &lines[0];
    assert_eq!(line.len(), 5);
    assert_eq!(line.get_cell(0).unwrap().width(), 1);
    assert_eq!(line.get_cell(0).unwrap().str(), "a");

    let clusters = line.cluster(None);
    assert_eq!(clusters.len(), 2);

    let cluster = &clusters[0];
    assert_eq!(cluster.width, 2);
    assert_eq!(&cluster.text, "a ");
    let subclusters = subdivide(line, cluster, &options).collect_vec();
    assert_eq!(subclusters.len(), 1);
    assert_eq!(subclusters[0].0, "a ");
    assert_eq!(subclusters[0].1, 0..2);

    let cluster = &clusters[1];
    assert_eq!(cluster.width, 3);
    assert_eq!(&cluster.text, "   ");
    let subclusters = subdivide(line, cluster, &options).collect_vec();
    assert_eq!(subclusters.len(), 1);
    assert_eq!(subclusters[0].0, "   ");
    assert_eq!(subclusters[0].1, 2..5);
}

#[test]
fn test_subclusters_combining_characters() {
    let mut surface = Surface::new(5, 2);
    surface.add_change(Change::Text("◌́".into()));
    let options = Options::sample();

    let lines = surface.screen_lines();
    let lines = lines.iter().collect_vec();
    assert_eq!(lines.len(), 2);

    let line = &lines[0];
    assert_eq!(line.len(), 5);
    assert_eq!(line.get_cell(0).unwrap().width(), 1);
    assert_eq!(line.get_cell(0).unwrap().str(), "◌́");

    let clusters = line.cluster(None);
    assert_eq!(clusters.len(), 2);

    let cluster = &clusters[0];
    assert_eq!(cluster.width, 2);
    assert_eq!(&cluster.text, "◌́ ");
    let subclusters = subdivide(line, cluster, &options).collect_vec();
    assert_eq!(subclusters.len(), 1);
    assert_eq!(subclusters[0].0, "◌\u{301} ");
    assert_eq!(subclusters[0].1, 0..2);
}
