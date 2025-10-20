#[allow(unused_imports)]
use crate::config::types::snap::*;

#[test]
fn test_snap_up_f32() {
    // Basic snap up
    let result = 99.5_f32.snap_up(5.0);
    assert_eq!(result, 100.0);

    let result = 80.1_f32.snap_up(4.0);
    assert_eq!(result, 84.0);

    // Exact multiple should stay same
    let result = 100.0_f32.snap_up(5.0);
    assert_eq!(result, 100.0);
}

#[test]
fn test_snap_up_f32_checked() {
    let result = 99.5_f32.checked_snap_up(5.0);
    assert_eq!(result, Some(100.0));

    let result = 50.5_f32.checked_snap_up(0.0);
    assert_eq!(result, None);

    let result = 50.5_f32.checked_snap_up(-1.0);
    assert_eq!(result, None);
}

#[test]
fn test_snap_up_f64() {
    let result = 99.5_f64.snap_up(5.0);
    assert_eq!(result, 100.0);

    let result = 80.1_f64.snap_up(4.0);
    assert_eq!(result, 84.0);

    let result = 100.0_f64.snap_up(5.0);
    assert_eq!(result, 100.0);
}

#[test]
fn test_snap_up_f64_checked() {
    let result = 99.5_f64.checked_snap_up(5.0);
    assert_eq!(result, Some(100.0));

    let result = 50.5_f64.checked_snap_up(0.0);
    assert_eq!(result, None);

    let result = 50.5_f64.checked_snap_up(-1.0);
    assert_eq!(result, None);
}

#[test]
fn test_snap_up_u16() {
    use crate::config::types::snap::SnapUp;

    let result = 99_u16.snap_up(5);
    assert_eq!(result, 100);

    let result = 80_u16.snap_up(4);
    assert_eq!(result, 80);

    let result = 81_u16.snap_up(4);
    assert_eq!(result, 84);
}

#[test]
fn test_snap_up_u16_checked() {
    use crate::config::types::snap::SnapUp;

    let result = 99_u16.checked_snap_up(5);
    assert_eq!(result, Some(100));

    let result = 50_u16.checked_snap_up(0);
    assert_eq!(result, None);
}
