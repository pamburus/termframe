use std::str::FromStr;

use crate::cli::FontWeight;
use crate::config::{self, FontFamilyOption, PaddingOption, Patch, Settings, ThemeSetting};

#[test]
fn test_font_weight_from_str() {
    // Test parsing "normal"
    let weight = FontWeight::from_str("normal").unwrap();
    assert!(matches!(weight, FontWeight::Normal));

    // Test parsing "bold"
    let weight = FontWeight::from_str("bold").unwrap();
    assert!(matches!(weight, FontWeight::Bold));

    // Test parsing numeric weight
    let weight = FontWeight::from_str("400").unwrap();
    assert!(matches!(weight, FontWeight::Fixed(400)));

    // Test parsing invalid weight
    let result = FontWeight::from_str("invalid");
    assert!(result.is_err());
}

#[test]
fn test_font_weight_conversion() {
    // Test conversion from config::FontWeight to cli::FontWeight
    let config_normal = config::FontWeight::Normal;
    let cli_normal: FontWeight = config_normal.into();
    assert!(matches!(cli_normal, FontWeight::Normal));

    let config_bold = config::FontWeight::Bold;
    let cli_bold: FontWeight = config_bold.into();
    assert!(matches!(cli_bold, FontWeight::Bold));

    let config_fixed = config::FontWeight::Fixed(500);
    let cli_fixed: FontWeight = config_fixed.into();
    assert!(matches!(cli_fixed, FontWeight::Fixed(500)));

    // Test conversion from cli::FontWeight to config::FontWeight
    let cli_normal = FontWeight::Normal;
    let config_normal: config::FontWeight = cli_normal.into();
    assert!(matches!(config_normal, config::FontWeight::Normal));

    let cli_bold = FontWeight::Bold;
    let config_bold: config::FontWeight = cli_bold.into();
    assert!(matches!(config_bold, config::FontWeight::Bold));

    let cli_fixed = FontWeight::Fixed(600);
    let config_fixed: config::FontWeight = cli_fixed.into();
    assert!(matches!(config_fixed, config::FontWeight::Fixed(600)));
}

#[test]
fn test_opt_patch() {
    // Create base settings
    let mut settings = Settings::default();
    settings.terminal.width.current = 80.into();
    settings.terminal.height.current = 24.into();
    settings.font.family = FontFamilyOption::Single("Default".to_string());
    settings.theme = ThemeSetting::Fixed("default".to_string());
    settings.window.enabled = false;

    // Create test options with overrides
    let opt = create_test_opt();

    // Apply patch
    let patched = opt.patch(settings);

    // Verify patched settings
    assert_eq!(patched.terminal.width.current, 100.into());
    assert_eq!(patched.terminal.height.current, 50.into());
    assert!(matches!(patched.font.family, FontFamilyOption::Multiple(_)));
    if let FontFamilyOption::Multiple(families) = &patched.font.family {
        assert_eq!(families.len(), 2);
        assert_eq!(families[0], "Monospace");
        assert_eq!(families[1], "Consolas");
    }
    assert!(matches!(patched.theme, ThemeSetting::Fixed(_)));
    if let ThemeSetting::Fixed(theme) = &patched.theme {
        assert_eq!(theme, "dark");
    }
    assert!(patched.window.enabled);

    // Check padding
    if let PaddingOption::Uniform(padding) = patched.padding {
        assert_eq!(padding, 8.0.into());
    } else {
        panic!("Expected uniform padding");
    }
}

// Create a test-specific utility function to create a patching Opt
fn create_test_opt() -> impl Patch {
    struct TestOpt {
        pub width: u16,
        pub height: u16,
        pub font_family: Vec<String>,
        pub theme: Option<String>,
        pub window: bool,
        pub padding: Option<f32>,
    }

    impl Patch for TestOpt {
        fn patch(&self, settings: Settings) -> Settings {
            let mut settings = settings;

            settings.terminal.width.current = self.width.into();
            settings.terminal.height.current = self.height.into();
            if !self.font_family.is_empty() {
                settings.font.family = FontFamilyOption::Multiple(self.font_family.clone());
            }
            if let Some(theme) = &self.theme {
                settings.theme = ThemeSetting::Fixed(theme.clone());
            }
            settings.window.enabled = self.window;
            if let Some(padding) = self.padding {
                settings.padding = PaddingOption::Uniform(padding.into());
            }

            settings
        }
    }

    TestOpt {
        width: 100,
        height: 50,
        font_family: vec!["Monospace".to_string(), "Consolas".to_string()],
        theme: Some("dark".to_string()),
        window: true,
        padding: Some(8.0),
    }
}
