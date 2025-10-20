use super::*;
use crate::config::theme::ThemeConfig;

#[test]
fn test_from_buf_yaml_format() {
    // Test YAML format parsing to cover line 175
    let yaml_data = b"---\ntags: []\ntheme:\n  colors:\n    background: \"#000000\"\n    foreground: \"#ffffff\"\n    palette: {}";
    let result = ThemeConfig::from_buf(yaml_data, Format::Yaml);
    assert!(result.is_ok());
}
