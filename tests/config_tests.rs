use termframe::config::{
    self, FontFamilyOption, FontWeight, Number, PaddingOption, Settings, ThemeSetting,
};

#[test]
fn test_default_settings() {
    // Test that default settings can be loaded
    let settings = Settings::default();
    assert_eq!(settings.terminal.width.min(), 80.into());
    assert_eq!(settings.terminal.height.min(), 24.into());
}

#[test]
fn test_padding_option_resolve() {
    // Test uniform padding
    let uniform = PaddingOption::Uniform(Number::from(5.0));
    let padding = uniform.resolve();
    assert_eq!(padding.top, Number::from(5.0));
    assert_eq!(padding.bottom, Number::from(5.0));
    assert_eq!(padding.left, Number::from(5.0));
    assert_eq!(padding.right, Number::from(5.0));

    // Test symmetric padding
    let symmetric = PaddingOption::Symmetric {
        vertical: Number::from(2.0),
        horizontal: Number::from(3.0),
    };
    let padding = symmetric.resolve();
    assert_eq!(padding.top, Number::from(2.0));
    assert_eq!(padding.bottom, Number::from(2.0));
    assert_eq!(padding.left, Number::from(3.0));
    assert_eq!(padding.right, Number::from(3.0));

    // Test asymmetric padding
    let asymmetric = PaddingOption::Asymmetric(config::Padding {
        top: Number::from(1.0),
        bottom: Number::from(2.0),
        left: Number::from(3.0),
        right: Number::from(4.0),
    });
    let padding = asymmetric.resolve();
    assert_eq!(padding.top, Number::from(1.0));
    assert_eq!(padding.bottom, Number::from(2.0));
    assert_eq!(padding.left, Number::from(3.0));
    assert_eq!(padding.right, Number::from(4.0));
}

#[test]
fn test_font_family_option() {
    // Test single font family
    let single = FontFamilyOption::Single("Monospace".to_string());
    assert_eq!(single.primary(), "Monospace");
    assert_eq!(single.resolve(), vec!["Monospace".to_string()]);
    assert!(single.contains("Monospace"));
    assert!(!single.contains("Other"));

    // Test multiple font families
    let multiple = FontFamilyOption::Multiple(vec![
        "Monospace".to_string(),
        "Courier".to_string(),
        "Consolas".to_string(),
    ]);
    assert_eq!(multiple.primary(), "Monospace");
    assert_eq!(
        multiple.resolve(),
        vec![
            "Monospace".to_string(),
            "Courier".to_string(),
            "Consolas".to_string()
        ]
    );
    assert!(multiple.contains("Monospace"));
    assert!(multiple.contains("Courier"));
    assert!(multiple.contains("Consolas"));
    assert!(!multiple.contains("Other"));
}

#[test]
fn test_theme_setting_resolve() {
    // Test fixed theme
    let fixed = ThemeSetting::Fixed("dark".to_string());
    assert_eq!(fixed.resolve(config::mode::Mode::Light), "dark");
    assert_eq!(fixed.resolve(config::mode::Mode::Dark), "dark");

    // Test adaptive theme
    let adaptive = ThemeSetting::Adaptive {
        light: "light-theme".to_string(),
        dark: "dark-theme".to_string(),
    };
    assert_eq!(adaptive.resolve(config::mode::Mode::Light), "light-theme");
    assert_eq!(adaptive.resolve(config::mode::Mode::Dark), "dark-theme");
}

#[test]
fn test_font_weight_display() {
    assert_eq!(FontWeight::Normal.to_string(), "normal");
    assert_eq!(FontWeight::Bold.to_string(), "bold");
    assert_eq!(FontWeight::Fixed(400).to_string(), "400");
    assert_eq!(FontWeight::Fixed(700).to_string(), "700");
}

#[test]
fn test_padding_mul() {
    let padding = config::Padding {
        top: Number::from(1.0),
        bottom: Number::from(2.0),
        left: Number::from(3.0),
        right: Number::from(4.0),
    };

    // Test multiplication
    let result = padding * 2.0;
    assert_eq!(result.top, Number::from(2.0));
    assert_eq!(result.bottom, Number::from(4.0));
    assert_eq!(result.left, Number::from(6.0));
    assert_eq!(result.right, Number::from(8.0));

    // Test multiplication assign
    let mut padding = padding;
    padding *= 2.0;
    assert_eq!(padding.top, Number::from(2.0));
    assert_eq!(padding.bottom, Number::from(4.0));
    assert_eq!(padding.left, Number::from(6.0));
    assert_eq!(padding.right, Number::from(8.0));
}

#[test]
fn test_global_config() {
    // Initialize with custom settings
    let mut settings = Settings::default();
    settings.terminal.width = 100.into();
    settings.terminal.height = 40.into();

    config::global::initialize(settings.clone());

    // Get should return our custom settings
    let global_settings = config::global::get();
    assert_eq!(global_settings.terminal.width, 100.into());
    assert_eq!(global_settings.terminal.height, 40.into());
}

// Skip test_custom_source_loading as it requires access to internal FileFormat
