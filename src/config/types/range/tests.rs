#[allow(unused_imports)]
use crate::config::types::range::*;

#[test]
fn test_partial_range_display_both_bounds() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    assert_eq!(range.to_string(), "80..240");
}

#[test]
fn test_partial_range_display_max_only() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(None, Some(240u16));
    assert_eq!(range.to_string(), "..240");
}

#[test]
fn test_partial_range_display_min_only() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), None);
    assert_eq!(range.to_string(), "80..");
}

#[test]
fn test_partial_range_display_no_bounds() {
    use crate::config::types::range::PartialRange;

    let range: PartialRange<u16> = PartialRange::default();
    assert_eq!(range.to_string(), "..");
}

#[test]
fn test_partial_range_fit() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange {
        min: Some(50u16),
        max: Some(150u16),
    };

    assert_eq!(range.fit(40u16), 50u16);
    assert_eq!(range.fit(100u16), 100u16);
    assert_eq!(range.fit(200u16), 150u16);
}

#[test]
fn test_partial_range_from_str_invalid() {
    use crate::config::types::range::PartialRange;

    // Missing dots should error
    let result: Result<PartialRange<u16>, _> = "80 120".parse();
    assert!(result.is_err());

    // Multiple dots should error
    let result: Result<PartialRange<u16>, _> = "80...120".parse();
    assert!(result.is_err());

    // Invalid number should error
    let result: Result<PartialRange<u16>, _> = "abc..120".parse();
    assert!(result.is_err());
}

#[test]
fn test_partial_range_from_str_multiple_dots() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::range::RangeParseError;
    // Multiple ".." should error with InvalidFormat (80....120 has ".." twice)
    let result: Result<PartialRange<u16>, _> = "80....120".parse();
    match result {
        Err(RangeParseError::InvalidFormat) => {}
        other => panic!("expected InvalidFormat error, got {:?}", other),
    }
}

#[test]
fn test_partial_range_from_tuple() {
    use crate::config::types::range::PartialRange;

    let range: PartialRange<u16> = (80, 120).into();
    assert_eq!(range.min, Some(80));
    assert_eq!(range.max, Some(120));
}

#[test]
fn test_partial_range_max_method() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    assert_eq!(range.max(), Some(240));

    let range: PartialRange<u16> = PartialRange::default();
    assert_eq!(range.max(), None);
}

#[test]
fn test_partial_range_min_method() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    assert_eq!(range.min(), Some(80));

    let range: PartialRange<u16> = PartialRange::default();
    assert_eq!(range.min(), None);
}

#[test]
fn test_partial_range_new() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    assert_eq!(range.min, Some(80));
    assert_eq!(range.max, Some(240));
}

#[test]
fn test_partial_range_parse_error_multiple_dots_error() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::range::RangeParseError;
    // Verify the error is InvalidFormat when we have ".." appearing twice (80....120)
    let result: Result<PartialRange<u16>, _> = "80....120".parse();
    match result {
        Err(RangeParseError::InvalidFormat) => {}
        other => panic!("expected InvalidFormat error, got {:?}", other),
    }
}

#[test]
fn test_partial_range_range_bounds() {
    use crate::config::types::range::PartialRange;
    use std::ops::RangeBounds;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    assert!(range.contains(&100u16));
    assert!(!range.contains(&300u16));
    assert!(!range.contains(&50u16));
}

#[test]
fn test_partial_range_range_bounds_included_end() {
    use crate::config::types::range::PartialRange;
    use std::ops::RangeBounds;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    match range.end_bound() {
        std::ops::Bound::Included(val) => assert_eq!(*val, 240u16),
        _ => panic!("expected Included"),
    }
}

#[test]
fn test_partial_range_range_bounds_included_start() {
    use crate::config::types::range::PartialRange;
    use std::ops::RangeBounds;

    let range = PartialRange::new(Some(80u16), Some(240u16));
    match range.start_bound() {
        std::ops::Bound::Included(val) => assert_eq!(*val, 80u16),
        _ => panic!("expected Included"),
    }
}

#[test]
fn test_partial_range_range_bounds_unbounded_end() {
    use crate::config::types::range::PartialRange;
    use std::ops::RangeBounds;

    let range = PartialRange::new(Some(80u16), None);
    match range.end_bound() {
        std::ops::Bound::Unbounded => {}
        _ => panic!("expected Unbounded"),
    }
}

#[test]
fn test_partial_range_range_bounds_unbounded_start() {
    use crate::config::types::range::PartialRange;
    use std::ops::RangeBounds;

    let range = PartialRange::new(None, Some(240u16));
    match range.start_bound() {
        std::ops::Bound::Unbounded => {}
        _ => panic!("expected Unbounded"),
    }
}

#[test]
fn test_partial_range_with_max() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(Some(80u16), None);
    let with_max = range.with_max(240u16);
    assert_eq!(with_max.min, Some(80));
    assert_eq!(with_max.max, Some(240));
}

#[test]
fn test_partial_range_with_min() {
    use crate::config::types::range::PartialRange;

    let range = PartialRange::new(None, Some(240u16));
    let with_min = range.with_min(80u16);
    assert_eq!(with_min.min, Some(80));
    assert_eq!(with_min.max, Some(240));
}

#[test]
fn test_range_parse_error_display_bound_parse_error() {
    use crate::config::types::range::RangeParseError;

    let err: RangeParseError<&str> = RangeParseError::BoundParseError("invalid number");
    assert_eq!(err.to_string(), "bound parse error: invalid number");
}

#[test]
fn test_range_parse_error_display_invalid_format() {
    use crate::config::types::range::RangeParseError;

    let err: RangeParseError<String> = RangeParseError::InvalidFormat;
    assert_eq!(err.to_string(), "invalid range format");
}

#[test]
fn test_range_parse_error_display_missing_dots() {
    use crate::config::types::range::RangeParseError;

    let err: RangeParseError<String> = RangeParseError::MissingDots;
    assert_eq!(err.to_string(), "expected range syntax with '..'");
}

#[test]
fn test_stepped_range_from_partial_range() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let partial = PartialRange {
        min: Some(80u16),
        max: Some(120u16),
    };

    let stepped: SteppedRange<u16> = partial.into();
    assert_eq!(stepped.min(), Some(80));
    assert_eq!(stepped.max(), Some(120));
    assert_eq!(stepped.step(), None);
}

#[test]
fn test_partial_range_from_str_invalid_min() {
    use crate::config::types::range::{PartialRange, RangeParseError};

    let result: Result<PartialRange<u16>, _> = "abc..120".parse();
    match result {
        Err(RangeParseError::BoundParseError(_)) => {}
        other => panic!("expected BoundParseError, got {:?}", other),
    }
}

#[test]
fn test_partial_range_from_str_invalid_max() {
    use crate::config::types::range::{PartialRange, RangeParseError};

    let result: Result<PartialRange<u16>, _> = "80..abc".parse();
    match result {
        Err(RangeParseError::BoundParseError(_)) => {}
        other => panic!("expected BoundParseError, got {:?}", other),
    }
}

#[test]
fn test_partial_range_from_str_max_only_parse() {
    use crate::config::types::range::PartialRange;

    let result: Result<PartialRange<u16>, _> = "..240".parse();
    assert!(result.is_ok());
    let range = result.unwrap();
    assert_eq!(range.min, None);
    assert_eq!(range.max, Some(240u16));
}

#[test]
fn test_partial_range_from_str_min_only_parse() {
    use crate::config::types::range::PartialRange;

    let result: Result<PartialRange<u16>, _> = "80..".parse();
    assert!(result.is_ok());
    let range = result.unwrap();
    assert_eq!(range.min, Some(80u16));
    assert_eq!(range.max, None);
}
