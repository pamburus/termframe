#[allow(unused_imports)]
use crate::config::types::dimension::*;

#[test]
fn test_dimension_auto_fit() {
    let dim: Dimension<u16> = Dimension::Auto;
    // Auto dimensions pass through the value unchanged
    assert_eq!(dim.fit(50u16), 50);
    assert_eq!(dim.fit(100u16), 100);
    assert_eq!(dim.fit(200u16), 200);
}

#[test]
fn test_dimension_auto_max() {
    let dim: Dimension<u16> = Dimension::Auto;
    assert_eq!(dim.max(), None);
}

#[test]
fn test_dimension_auto_min() {
    let dim: Dimension<u16> = Dimension::Auto;
    assert_eq!(dim.min(), None);
}

#[test]
fn test_dimension_auto_range() {
    let dim: Dimension<u16> = Dimension::Auto;
    let range = dim.range();
    assert_eq!(range.min(), None);
    assert_eq!(range.max(), None);
    assert_eq!(range.step(), None);
}

#[test]
fn test_dimension_auto_step() {
    let dim: Dimension<u16> = Dimension::Auto;
    assert_eq!(dim.step(), None);
}

#[test]
fn test_dimension_display() {
    let dim: Dimension<u16> = Dimension::Auto;
    assert_eq!(dim.to_string(), "auto");

    let dim = Dimension::Fixed(100u16);
    assert_eq!(dim.to_string(), "100");
}

#[test]
fn test_dimension_display_range_with_step() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim = Dimension::Limited(stepped_range);
    let display = dim.to_string();
    assert_eq!(display, "80..240:4");
}

#[test]
fn test_dimension_fit_with_step() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(100u16),
        },
        step: Some(5u16),
    };
    let dim = Dimension::Limited(stepped_range);

    // 99 should snap up to 100 with step 5
    let result = dim.fit(99u16);
    assert_eq!(result, 100);

    // 75 should snap up to 80 and be constrained by min
    let result = dim.fit(75u16);
    assert_eq!(result, 80);
}

#[test]
fn test_dimension_fixed_fit() {
    let dim = Dimension::Fixed(100u16);
    // Fixed dimensions always return their fixed value
    assert_eq!(dim.fit(50u16), 100);
    assert_eq!(dim.fit(100u16), 100);
    assert_eq!(dim.fit(200u16), 100);
}

#[test]
fn test_dimension_fixed_max() {
    let dim = Dimension::Fixed(100u16);
    assert_eq!(dim.max(), Some(100));
}

#[test]
fn test_dimension_fixed_min() {
    let dim = Dimension::Fixed(100u16);
    assert_eq!(dim.min(), Some(100));
}

#[test]
fn test_dimension_fixed_range() {
    let dim = Dimension::Fixed(100u16);
    let range = dim.range();
    assert_eq!(range.min(), Some(100));
    assert_eq!(range.max(), Some(100));
    assert_eq!(range.step(), None);
}

#[test]
fn test_dimension_fixed_step() {
    let dim = Dimension::Fixed(100u16);
    assert_eq!(dim.step(), None);
}

#[test]
fn test_dimension_from_stepped_range() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim: Dimension<u16> = stepped_range.into();
    assert!(matches!(dim, Dimension::Limited(_)));
    assert_eq!(dim.min(), Some(80));
    assert_eq!(dim.max(), Some(240));
    assert_eq!(dim.step(), Some(4));
}

#[test]
fn test_dimension_from_str() {
    let dim: Dimension<u16> = Dimension::from_str("auto").unwrap();
    assert_eq!(dim, Dimension::Auto);

    let dim: Dimension<u16> = Dimension::from_str("100").unwrap();
    assert_eq!(dim, Dimension::Fixed(100));

    let dim: Dimension<u16> = Dimension::from_str("80..120").unwrap();
    match dim {
        Dimension::Limited(sr) => {
            assert_eq!(sr.min(), Some(80));
            assert_eq!(sr.max(), Some(120));
        }
        _ => panic!("expected Limited"),
    }
}

#[test]
fn test_dimension_from_str_invalid() {
    // Invalid number should error
    let result: Result<Dimension<u16>, _> = "abc".parse();
    assert!(result.is_err());

    // Invalid range format should error
    let result: Result<Dimension<u16>, _> = "abc..120".parse();
    assert!(result.is_err());
}

#[test]
fn test_dimension_limited_max() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim = Dimension::Limited(stepped_range);
    assert_eq!(dim.max(), Some(240));
}

#[test]
fn test_dimension_limited_min() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim = Dimension::Limited(stepped_range);
    assert_eq!(dim.min(), Some(80));
}

#[test]
fn test_dimension_limited_range() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim = Dimension::Limited(stepped_range);
    let range = dim.range();
    assert_eq!(range.min(), Some(80));
    assert_eq!(range.max(), Some(240));
    assert_eq!(range.step(), Some(4));
}

#[test]
fn test_dimension_limited_step() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped_range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: Some(4u16),
    };
    let dim = Dimension::Limited(stepped_range);
    assert_eq!(dim.step(), Some(4));
}

#[test]
fn test_dimension_with_min_and_max() {
    let dim = Dimension::<u16>::Auto;
    let with_min = dim.with_min(80);
    assert_eq!(with_min.min(), Some(80));

    let with_max = dim.with_max(120);
    assert_eq!(with_max.max(), Some(120));
}
