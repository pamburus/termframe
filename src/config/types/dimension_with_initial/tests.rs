#[allow(unused_imports)]
use crate::config::types::dimension_with_initial::*;

#[test]
fn test_dimension_with_initial_auto_max() {
    let dwi = DimensionWithInitial {
        current: Dimension::<u16>::Auto,
        initial: None,
    };
    assert_eq!(dwi.max(), None);
}

#[test]
fn test_dimension_with_initial_auto_min() {
    let dwi = DimensionWithInitial {
        current: Dimension::<u16>::Auto,
        initial: None,
    };
    assert_eq!(dwi.min(), None);
}

#[test]
fn test_dimension_with_initial_auto_range() {
    let dwi = DimensionWithInitial {
        current: Dimension::<u16>::Auto,
        initial: None,
    };
    let range = dwi.range();
    assert_eq!(range.min(), None);
    assert_eq!(range.max(), None);
    assert_eq!(range.step(), None);
}

#[test]
fn test_dimension_with_initial_auto_step() {
    let dwi = DimensionWithInitial {
        current: Dimension::<u16>::Auto,
        initial: None,
    };
    assert_eq!(dwi.step(), None);
}

#[test]
fn test_dimension_with_initial_display() {
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: Some(80u16),
    };
    assert_eq!(dwi.to_string(), "100@80");

    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: None,
    };
    assert_eq!(dwi.to_string(), "100");
}

#[test]
fn test_dimension_with_initial_display_with_range_and_step() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(160u16),
    };
    let display = dwi.to_string();
    assert_eq!(display, "80..240:4@160");
}

#[test]
fn test_dimension_with_initial_fit() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(100u16),
        },
        step: Some(5u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: None,
    };

    assert_eq!(dwi.fit(99u16), 100);
    assert_eq!(dwi.fit(50u16), 80);
}

#[test]
fn test_dimension_with_initial_fixed_max() {
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: None,
    };
    assert_eq!(dwi.max(), Some(100));
}

#[test]
fn test_dimension_with_initial_fixed_min() {
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: None,
    };
    assert_eq!(dwi.min(), Some(100));
}

#[test]
fn test_dimension_with_initial_fixed_range() {
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: None,
    };
    let range = dwi.range();
    assert_eq!(range.min(), Some(100));
    assert_eq!(range.max(), Some(100));
    assert_eq!(range.step(), None);
}

#[test]
fn test_dimension_with_initial_fixed_step() {
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: None,
    };
    assert_eq!(dwi.step(), None);
}

#[test]
fn test_dimension_with_initial_from_conversion() {
    use crate::config::types::Dimension;

    // Test From<DimensionWithInitial<T>> to Dimension<T> conversion (src/config.rs:29-31)
    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(100u16),
        initial: Some(80u16),
    };
    let dim: Dimension<u16> = dwi.into();
    assert_eq!(dim, Dimension::Fixed(100u16));

    let dwi = DimensionWithInitial {
        current: Dimension::Auto,
        initial: None,
    };
    let dim: Dimension<u16> = dwi.into();
    assert_eq!(dim, Dimension::Auto);
}

#[test]
fn test_dimension_with_initial_from_dimension() {
    // Test From<Dimension<T>> implementation
    let dim = Dimension::Fixed(100u16);
    let dwi: DimensionWithInitial<u16> = dim.into();
    assert_eq!(dwi.current, Dimension::Fixed(100));
    assert_eq!(dwi.initial, None);

    let dim = Dimension::Auto;
    let dwi: DimensionWithInitial<u16> = dim.into();
    assert_eq!(dwi.current, Dimension::Auto);
    assert_eq!(dwi.initial, None);
}

#[test]
fn test_dimension_with_initial_from_str() {
    let dwi: DimensionWithInitial<u16> = DimensionWithInitial::from_str("80..120@100").unwrap();
    assert_eq!(dwi.initial, Some(100));

    let dwi: DimensionWithInitial<u16> = DimensionWithInitial::from_str("@160").unwrap();
    assert!(matches!(dwi.current, Dimension::Auto));
    assert_eq!(dwi.initial, Some(160));

    let dwi: DimensionWithInitial<u16> = DimensionWithInitial::from_str("100").unwrap();
    assert_eq!(dwi.initial, None);
}

#[test]
fn test_dimension_with_initial_from_str_invalid() {
    // Invalid number should error
    let result: Result<DimensionWithInitial<u16>, _> = "abc@100".parse();
    assert!(result.is_err());

    // Invalid initial should error
    let result: Result<DimensionWithInitial<u16>, _> = "100@abc".parse();
    assert!(result.is_err());
}

#[test]
fn test_dimension_with_initial_from_value() {
    // Test From<T> implementation
    let dwi: DimensionWithInitial<u16> = 50u16.into();
    assert_eq!(dwi.current, Dimension::Fixed(50));
    assert_eq!(dwi.initial, None);
}

#[test]
fn test_dimension_with_initial_initial_or() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(100u16),
        },
        step: Some(5u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(99u16),
    };

    // Should return fitted initial value (99 snaps up to 100 with step 5)
    assert_eq!(dwi.initial_or(0u16), 100);

    let dwi = DimensionWithInitial {
        current: Dimension::Fixed(90u16),
        initial: None,
    };

    // Should return fixed value
    assert_eq!(dwi.initial_or(0u16), 90);

    let dwi = DimensionWithInitial {
        current: Dimension::Auto,
        initial: None,
    };

    // Should return fitted fallback
    assert_eq!(dwi.initial_or(42u16), 42);
}

#[test]
fn test_dimension_with_initial_limited_range() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(160u16),
    };
    let range = dwi.range();
    assert_eq!(range.min(), Some(80));
    assert_eq!(range.max(), Some(240));
    assert_eq!(range.step(), Some(4));
}

#[test]
fn test_dimension_with_initial_max_method() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(160u16),
    };
    assert_eq!(dwi.max(), Some(240));
}

#[test]
fn test_dimension_with_initial_min_method() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(160u16),
    };
    assert_eq!(dwi.min(), Some(80));
}

#[test]
fn test_dimension_with_initial_step_method() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dwi = DimensionWithInitial {
        current: Dimension::Limited(stepped_range),
        initial: Some(160u16),
    };
    assert_eq!(dwi.step(), Some(4));
}

#[test]
fn test_dimension_with_initial_deserialize_simple_fixed() {
    let json_str = r#"100"#;
    let result: Result<DimensionWithInitial<u16>, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
    let dwi = result.unwrap();
    assert_eq!(dwi.current, Dimension::Fixed(100u16));
    assert_eq!(dwi.initial, None);
}

#[test]
fn test_dimension_with_initial_deserialize_spec_with_constraints_and_initial() {
    let json_str = r#"{"min":80,"max":240,"step":4,"initial":160}"#;
    let result: Result<DimensionWithInitial<u16>, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
    let dwi = result.unwrap();
    match dwi.current {
        Dimension::Limited(sr) => {
            assert_eq!(sr.range.min, Some(80));
            assert_eq!(sr.range.max, Some(240));
            assert_eq!(sr.step, Some(4));
        }
        _ => panic!("expected Limited range"),
    }
    assert_eq!(dwi.initial, Some(160));
}

#[test]
fn test_dimension_with_initial_deserialize_spec_only_initial() {
    let json_str = r#"{"initial":160}"#;
    let result: Result<DimensionWithInitial<u16>, _> = serde_json::from_str(json_str);
    assert!(result.is_ok());
    let dwi = result.unwrap();
    assert_eq!(dwi.current, Dimension::Auto);
    assert_eq!(dwi.initial, Some(160));
}

#[test]
fn test_dimension_with_initial_from_str_with_empty_initial() {
    let dwi: DimensionWithInitial<u16> = DimensionWithInitial::from_str("100@").unwrap();
    assert_eq!(dwi.current, Dimension::Fixed(100u16));
    assert_eq!(dwi.initial, None);
}
