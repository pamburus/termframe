use super::*;

#[test]
fn test_optimize_contour() {
    let contour = vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)];
    let optimized = optimize_contour(contour.clone());
    assert_eq!(optimized, vec![(0, 0), (1, 0), (1, 1), (0, 1)]);

    let contour = vec![(0, 0), (2, 0), (2, 2), (0, 2), (0, 0)];
    let optimized = optimize_contour(contour.clone());
    assert_eq!(optimized, vec![(0, 0), (2, 0), (2, 2), (0, 2)]);

    let contour = vec![
        (0, 0),
        (1, 0),
        (2, 0),
        (2, 1),
        (2, 2),
        (1, 2),
        (0, 2),
        (0, 1),
        (0, 0),
    ];
    let optimized = optimize_contour(contour.clone());
    assert_eq!(optimized, vec![(0, 0), (2, 0), (2, 2), (0, 2)]);
}

#[test]
fn test_find_clusters() {
    let cols = 3;
    let rows = 3;
    let group = |x, y| {
        if (x == 1 && y == 1) || (x == 2 && y == 2) {
            Some(1)
        } else {
            None
        }
    };
    let clusters = find_clusters(cols, rows, group);
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].0, 1);
    assert_eq!(clusters[0].1, vec![(1, 1)]);
    assert_eq!(clusters[1].0, 1);
    assert_eq!(clusters[1].1, vec![(2, 2)]);
}

#[test]
fn test_create_mask() {
    let cluster = vec![(0, 0), (1, 1)];
    let mask = create_mask(&cluster, 3, 3);
    assert!(mask.get(0, 0));
    assert!(mask.get(1, 1));
    assert!(!mask.get(2, 2));
}

#[test]
fn test_extract_boundary_segments() {
    let cluster = vec![(0, 0), (1, 0)];
    let mask = create_mask(&cluster, 3, 3);
    let segments = extract_boundary_segments(&mask);
    assert_eq!(segments.len(), 6);
}

#[test]
fn test_group_segments_into_contours() {
    let segments = vec![
        ((0, 0), (1, 0)),
        ((1, 0), (1, 1)),
        ((1, 1), (0, 1)),
        ((0, 1), (0, 0)),
    ];
    let contours = group_segments_into_contours(segments);
    assert_eq!(contours.len(), 1);
    assert_eq!(contours[0], vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]);
}

#[test]
fn test_signed_area() {
    let contour = vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)];
    let area = signed_area(&contour);
    assert_eq!(area, 2);
}

#[test]
fn test_reorient_contours() {
    let contours = vec![
        vec![(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)],
        vec![(0, 0), (2, 0), (2, 2), (0, 2), (0, 0)],
    ];
    let reoriented = reorient_contours(contours);
    assert_eq!(
        reoriented,
        [
            [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)],
            [(0, 0), (2, 0), (2, 2), (0, 2), (0, 0)]
        ],
    );
}
