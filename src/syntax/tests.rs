use super::*;

#[test]
fn test_theme_parse_with_dash() {
    let theme = Theme::parse("aura-dark").unwrap();
    assert_eq!(theme.display_name(), "aura-dark");
}

#[test]
fn test_theme_parse_with_underscore() {
    let theme = Theme::parse("aura_dark").unwrap();
    assert_eq!(theme.display_name(), "aura_dark");
}

#[test]
fn test_theme_from_str() {
    let theme: Theme = "dracula".parse().unwrap();
    assert_eq!(theme.display_name(), "dracula");
}

#[test]
fn test_theme_parse_error() {
    let result = Theme::parse("nonexistent-theme-12345");
    assert!(result.is_err());
}

#[test]
fn test_available_themes_returns_dash_separator() {
    let themes: Vec<_> = available_themes().collect();
    assert!(!themes.is_empty());

    for theme in &themes {
        assert!(
            !theme.name.contains('_'),
            "Theme '{}' contains underscore",
            theme.name
        );
        if theme.name.contains('-') {
            assert!(!theme.name.starts_with('-'));
            assert!(!theme.name.ends_with('-'));
        }
    }
}

#[test]
fn test_available_themes_contains_expected() {
    let themes: Vec<_> = available_themes().map(|t| t.name).collect();
    assert!(themes.iter().any(|name| name.contains("aura")));
    assert!(themes.iter().any(|name| name.contains("dracula")));
}

#[test]
fn test_appearance_display() {
    assert_eq!(Appearance::Dark.to_string(), "dark");
    assert_eq!(Appearance::Light.to_string(), "light");
}

#[test]
fn test_highlighter_new() {
    let theme = Theme::parse("dracula").unwrap();
    let highlighter = Highlighter::new(Language::Bash, Some(theme));

    let mut output = Vec::new();
    let result = highlighter.format("echo hello", &mut output);
    assert!(result.is_ok());
    assert!(!output.is_empty());
}

#[test]
fn test_highlighter_without_theme() {
    let highlighter = Highlighter::new(Language::Bash, None);

    let mut output = Vec::new();
    let result = highlighter.format("echo hello", &mut output);
    assert!(result.is_ok());
    assert!(!output.is_empty());
}

#[test]
fn test_theme_parse_complex_names() {
    let test_cases = vec![
        "aura-dark",
        "aura-dark-soft-text",
        "catppuccin-frappe",
        "catppuccin-macchiato",
    ];

    for name in test_cases {
        let theme = Theme::parse(name);
        assert!(theme.is_ok(), "Failed to parse theme: {}", name);
        assert_eq!(theme.unwrap().display_name(), name);
    }
}
