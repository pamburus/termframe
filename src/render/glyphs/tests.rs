use super::*;

const CELL: Cell = Cell {
    width: 0.5,
    height: 1.0,
};

fn rects(ch: char) -> Vec<Rect> {
    glyph(ch, CELL)
        .unwrap()
        .rects
        .into_iter()
        .map(round)
        .collect()
}

/// Rounds the coordinates to hide differences in the order of floating-point operations.
fn round(rect: Rect) -> Rect {
    let r = |v: f32| (v * 1e6).round() / 1e6;
    Rect::new(r(rect.x0), r(rect.y0), r(rect.x1), r(rect.y1))
}

#[test]
fn test_is_builtin() {
    for ch in ['─', '╬', '╿', '▀', '▟', '🬀', '🬻', '\u{1CD00}', '\u{1CDE5}'] {
        assert!(is_builtin(ch), "{ch:?}");
        assert!(glyph(ch, CELL).is_some(), "{ch:?}");
    }
    for ch in ['a', ' ', '⠿', '🬼', '\u{1CDE6}'] {
        assert!(!is_builtin(ch), "{ch:?}");
        assert!(glyph(ch, CELL).is_none(), "{ch:?}");
    }
}

#[test]
fn test_block_elements() {
    assert_eq!(rects('▀'), [round(Rect::new(0.0, 0.0, 0.5, 0.5))]);
    assert_eq!(rects('▄'), [round(Rect::new(0.0, 0.5, 0.5, 1.0))]);
    assert_eq!(rects('▁'), [round(Rect::new(0.0, 0.875, 0.5, 1.0))]);
    assert_eq!(rects('█'), [round(Rect::new(0.0, 0.0, 0.5, 1.0))]);
    assert_eq!(rects('▉'), [round(Rect::new(0.0, 0.0, 0.4375, 1.0))]);
    assert_eq!(rects('▏'), [round(Rect::new(0.0, 0.0, 0.0625, 1.0))]);
    assert_eq!(rects('▐'), [round(Rect::new(0.25, 0.0, 0.5, 1.0))]);
    assert_eq!(rects('▔'), [round(Rect::new(0.0, 0.0, 0.5, 0.125))]);
    assert_eq!(rects('▕'), [round(Rect::new(0.4375, 0.0, 0.5, 1.0))]);
    assert_eq!(
        rects('▚'),
        [
            round(Rect::new(0.0, 0.0, 0.25, 0.5)),
            round(Rect::new(0.25, 0.5, 0.5, 1.0))
        ]
    );

    for (ch, opacity) in [('░', 0.25), ('▒', 0.5), ('▓', 0.75), ('█', 1.0)] {
        let glyph = glyph(ch, CELL).unwrap();
        assert_eq!(glyph.opacity, opacity, "{ch:?}");
        assert_eq!(
            glyph.rects,
            [round(Rect::new(0.0, 0.0, 0.5, 1.0))],
            "{ch:?}"
        );
    }
}

#[test]
fn test_sextants() {
    assert_eq!(sextant(0), 0b000001);
    assert_eq!(sextant(19), 0b010100);
    assert_eq!(sextant(20), 0b010110);
    assert_eq!(sextant(39), 0b101001);
    assert_eq!(sextant(40), 0b101011);
    assert_eq!(sextant(59), 0b111110);

    // U+1FB15 is BLOCK SEXTANT-1235
    assert_eq!(
        rects('\u{1FB15}'),
        [
            round(Rect::new(0.0, 0.0, 0.25, 1.0 / 3.0)),
            round(Rect::new(0.25, 0.0, 0.5, 1.0 / 3.0)),
            round(Rect::new(0.0, 1.0 / 3.0, 0.25, 2.0 / 3.0)),
            round(Rect::new(0.0, 2.0 / 3.0, 0.25, 1.0)),
        ]
    );
}

#[test]
fn test_octants() {
    // U+1CD00 is BLOCK OCTANT-3, U+1CD01 is BLOCK OCTANT-23, U+1CDE5 is BLOCK OCTANT-2345678
    assert_eq!(OCTANTS[0], 0b00000100);
    assert_eq!(OCTANTS[1], 0b00000110);
    assert_eq!(OCTANTS[229], 0b11111110);
    assert!(OCTANTS.windows(2).all(|w| w[0] < w[1]));

    assert_eq!(rects('\u{1CD00}'), [round(Rect::new(0.0, 0.25, 0.25, 0.5))]);
}

#[test]
fn test_light_lines() {
    let (cx, cy) = (0.25, 0.5);
    let half = LIGHT / 2.0;

    // Straight line: two arms meeting in the center.
    assert_eq!(
        rects('─'),
        [
            round(Rect::new(cx, cy - half, 0.5, cy + half)),
            round(Rect::new(0.0, cy - half, cx, cy + half)),
        ]
    );

    // Corner: arms extend past the center to fill the corner.
    assert_eq!(
        rects('┌'),
        [
            round(Rect::new(cx - half, cy - half, 0.5, cy + half)),
            round(Rect::new(cx - half, cy - half, cx + half, 1.0)),
        ]
    );

    // Mixed weights: the light arm extends by half of the heavy perpendicular arm.
    assert_eq!(
        rects('┍'),
        [
            round(Rect::new(cx - half, cy - LIGHT, 0.5, cy + LIGHT)),
            round(Rect::new(cx - half, cy - LIGHT, cx + half, 1.0)),
        ]
    );

    // Half line.
    assert_eq!(
        rects('╴'),
        [round(Rect::new(0.0, cy - half, cx, cy + half))]
    );
}

#[test]
fn test_double_lines() {
    let (cx, cy) = (0.25, 0.5);
    let half = LIGHT / 2.0;
    let d = DOUBLE;

    // Outer stroke of a corner starts at the far stroke, inner one at the near stroke.
    assert_eq!(
        rects('╔'),
        [
            round(Rect::new(cx - d - half, cy - d - half, 0.5, cy - d + half)),
            round(Rect::new(cx + d - half, cy + d - half, 0.5, cy + d + half)),
            round(Rect::new(cx - d - half, cy - d - half, cx - d + half, 1.0)),
            round(Rect::new(cx + d - half, cy + d - half, cx + d + half, 1.0)),
        ]
    );

    // A through stroke continues from the opposite arm, the other one stops at the branch.
    assert_eq!(
        rects('╦')[..2],
        [
            round(Rect::new(cx, cy - d - half, 0.5, cy - d + half)),
            round(Rect::new(cx + d - half, cy + d - half, 0.5, cy + d + half)),
        ]
    );

    // Single arm at a double tee stops at the near stroke.
    assert_eq!(
        rects('╤')[4],
        round(Rect::new(cx - half, cy + d, cx + half, 1.0))
    );

    // Single arm at a double corner reaches the far stroke.
    assert_eq!(
        rects('╒')[2],
        round(Rect::new(cx - half, cy - d, cx + half, 1.0))
    );

    // Single arm crossing a double line goes through.
    assert_eq!(
        rects('╪')[4],
        round(Rect::new(cx - half, cy, cx + half, 1.0))
    );
}

#[test]
fn test_dashes() {
    let half = LIGHT / 2.0;
    assert_eq!(
        rects('╌'),
        [
            round(Rect::new(0.0625, 0.5 - half, 0.1875, 0.5 + half)),
            round(Rect::new(0.3125, 0.5 - half, 0.4375, 0.5 + half)),
        ]
    );
    assert_eq!(rects('┅').len(), 3);
    assert_eq!(rects('┋').len(), 4);
    assert_eq!(
        rects('┋')[0],
        round(Rect::new(0.25 - LIGHT, 0.0625, 0.25 + LIGHT, 0.1875))
    );
}

#[test]
fn test_arcs() {
    let half = LIGHT / 2.0;
    let radius = 0.25 - half;

    // Straight ends are rectangles reaching the cell edges, the curve stays within the cell.
    let arc = glyph('╭', CELL).unwrap();
    assert_eq!(
        arc.rects.into_iter().map(round).collect::<Vec<_>>(),
        [
            round(Rect::new(0.25 + radius, 0.5 - half, 0.5, 0.5 + half)),
            round(Rect::new(0.25 - half, 0.5 + radius, 0.25 + half, 1.0)),
        ]
    );
    assert_eq!(
        arc.curves,
        [Curve {
            start: (0.25 + radius + half, 0.5),
            segments: vec![
                Segment::Line((0.25 + radius, 0.5)),
                Segment::Arc {
                    to: (0.25, 0.5 + radius),
                    radius,
                    clockwise: false,
                },
                Segment::Line((0.25, 0.5 + radius + half)),
            ],
        }]
    );

    let curve = &glyph('╯', CELL).unwrap().curves[0];
    assert_eq!(curve.start, (0.25 - radius - half, 0.5));
    assert_eq!(
        curve.segments[1],
        Segment::Arc {
            to: (0.25, 0.5 - radius),
            radius,
            clockwise: false,
        }
    );

    for ch in ['╮', '╰'] {
        let curve = &glyph(ch, CELL).unwrap().curves[0];
        assert!(
            matches!(
                curve.segments[1],
                Segment::Arc {
                    clockwise: true,
                    ..
                }
            ),
            "{ch:?}"
        );
    }
}

#[test]
fn test_diagonals() {
    let (w, h) = (CELL.width, CELL.height);
    let half = LIGHT / 2.0;
    let len = (w * w + h * h).sqrt();
    // Lengths of the line cut by the horizontal and vertical cell edges.
    let (ex, ey) = (half * len / h, half * len / w);

    let sorted = |points: &[Point]| {
        let r = |v: f32| (v * 1e5).round() / 1e5;
        let mut points: Vec<_> = points.iter().map(|&(x, y)| (r(x), r(y))).collect();
        points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        points
    };

    let rising = sorted(&[
        (w - ex, 0.0),
        (w, 0.0),
        (w, ey),
        (ex, h),
        (0.0, h),
        (0.0, h - ey),
    ]);
    let falling = sorted(&[
        (0.0, 0.0),
        (ex, 0.0),
        (w, h - ey),
        (w, h),
        (w - ex, h),
        (0.0, ey),
    ]);

    let polygons = |ch| -> Vec<_> {
        let glyph = glyph(ch, CELL).unwrap();
        assert!(glyph.rects.is_empty() && glyph.curves.is_empty(), "{ch:?}");
        glyph.polygons.iter().map(|p| sorted(p)).collect()
    };

    assert_eq!(polygons('╱'), std::slice::from_ref(&rising));
    assert_eq!(polygons('╲'), std::slice::from_ref(&falling));
    assert_eq!(polygons('╳'), [rising, falling]);
}
