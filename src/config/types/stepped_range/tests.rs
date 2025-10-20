#[allow(unused_imports)]
use crate::config::types::stepped_range::*;

#[test]
fn test_stepped_range_fit_with_step() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(120u16),
        },
        step: Some(4u16),
    };

    // 99 should snap to 100 with step 4
    let result = stepped.fit(99u16);
    assert_eq!(result, 100);

    // 85 should snap to 88 with step 4
    let result = stepped.fit(85u16);
    assert_eq!(result, 88);
}

#[test]
fn test_stepped_range_from_str_invalid() {
    use crate::config::types::stepped_range::SteppedRange;

    // Invalid range format should error
    let result: Result<SteppedRange<u16>, _> = "abc..120".parse();
    assert!(result.is_err());

    // Invalid step should error
    let result: Result<SteppedRange<u16>, _> = "80..120:abc".parse();
    assert!(result.is_err());
}

#[test]
fn test_stepped_range_from_str_with_empty_step() {
    use crate::config::types::stepped_range::SteppedRange;

    let stepped: SteppedRange<u16> = "80..120:".parse().unwrap();
    assert_eq!(stepped.min(), Some(80));
    assert_eq!(stepped.max(), Some(120));
    assert_eq!(stepped.step(), None);
}

#[test]
fn test_stepped_range_methods() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let stepped = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(120u16),
        },
        step: Some(5u16),
    };

    assert_eq!(stepped.min(), Some(80));
    assert_eq!(stepped.max(), Some(120));
    assert_eq!(stepped.step(), Some(5));

    let as_range = stepped.as_range();
    assert_eq!(as_range.min, Some(80));
    assert_eq!(as_range.max, Some(120));

    let into_range = stepped.into_range();
    assert_eq!(into_range.min, Some(80));
    assert_eq!(into_range.max, Some(120));
}

#[test]
fn test_stepped_range_new() {
    use crate::config::types::stepped_range::SteppedRange;

    let range = SteppedRange::new(Some(80u16), Some(240u16), Some(4u16));
    assert_eq!(range.min(), Some(80));
    assert_eq!(range.max(), Some(240));
    assert_eq!(range.step(), Some(4));
}

#[test]
fn test_stepped_range_range_bounds() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;
    use std::ops::RangeBounds;

    let stepped = SteppedRange {
        range: PartialRange::new(Some(80u16), Some(240u16)),
        step: Some(4u16),
    };
    assert!(stepped.contains(&100u16));
    assert!(!stepped.contains(&300u16));
    assert!(!stepped.contains(&50u16));
}

#[test]
fn test_stepped_range_range_bounds_included_end() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;
    use std::ops::RangeBounds;

    let stepped = SteppedRange {
        range: PartialRange::new(Some(80u16), Some(240u16)),
        step: Some(4u16),
    };
    match stepped.end_bound() {
        std::ops::Bound::Included(val) => assert_eq!(*val, 240u16),
        _ => panic!("expected Included"),
    }
}

#[test]
fn test_stepped_range_range_bounds_included_start() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;
    use std::ops::RangeBounds;

    let stepped = SteppedRange {
        range: PartialRange::new(Some(80u16), Some(240u16)),
        step: Some(4u16),
    };
    match stepped.start_bound() {
        std::ops::Bound::Included(val) => assert_eq!(*val, 80u16),
        _ => panic!("expected Included"),
    }
}

#[test]
fn test_stepped_range_range_bounds_unbounded_end() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;
    use std::ops::RangeBounds;

    let stepped = SteppedRange {
        range: PartialRange::new(Some(80u16), None),
        step: Some(4u16),
    };
    match stepped.end_bound() {
        std::ops::Bound::Unbounded => {}
        _ => panic!("expected Unbounded"),
    }
}

#[test]
fn test_stepped_range_range_bounds_unbounded_start() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;
    use std::ops::RangeBounds;

    let stepped = SteppedRange {
        range: PartialRange::new(None, Some(240u16)),
        step: Some(4u16),
    };
    match stepped.start_bound() {
        std::ops::Bound::Unbounded => {}
        _ => panic!("expected Unbounded"),
    }
}

#[test]
fn test_stepped_range_with_max() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let range = SteppedRange {
        range: PartialRange::default(),
        step: None,
    };
    let with_max = range.with_max(240u16);
    assert_eq!(with_max.max(), Some(240));
}

#[test]
fn test_stepped_range_with_min() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let range = SteppedRange {
        range: PartialRange::default(),
        step: None,
    };
    let with_min = range.with_min(80u16);
    assert_eq!(with_min.min(), Some(80));
}

#[test]
fn test_stepped_range_with_step_builder() {
    use crate::config::types::range::PartialRange;
    use crate::config::types::stepped_range::SteppedRange;

    let range = SteppedRange {
        range: PartialRange {
            min: Some(80u16),
            max: Some(240u16),
        },
        step: None,
    };
    let with_step = range.with_step(4u16);
    assert_eq!(with_step.step(), Some(4));
    assert_eq!(with_step.min(), Some(80));
    assert_eq!(with_step.max(), Some(240));
}
