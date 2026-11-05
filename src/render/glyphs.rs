//! Shapes of block elements and box-drawing characters.
//!
//! These characters are drawn as shapes aligned to the cell grid instead of font glyphs,
//! so that adjacent characters join seamlessly regardless of the font and line height.
//! All coordinates are in em units, relative to the top-left corner of the cell.

/// Thickness of light lines.
pub const LIGHT: f32 = 1.0 / 12.0;

/// Thickness of heavy lines.
const HEAVY: f32 = LIGHT * 2.0;

/// Distance from the center of a double line to the center of each of its two strokes.
const DOUBLE: f32 = LIGHT;

pub type Point = (f32, f32);

/// Size of a cell.
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub width: f32,
    pub height: f32,
}

/// Shape of a character within its cell.
#[derive(Debug, Default, PartialEq)]
pub struct Glyph {
    /// Filled rectangles.
    pub rects: Vec<Rect>,
    /// Filled convex polygons.
    pub polygons: Vec<Vec<Point>>,
    /// Curves stroked with light line thickness.
    pub curves: Vec<Curve>,
    /// Opacity of the shape.
    pub opacity: f32,
}

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// Open curve made of straight and circular segments.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    pub start: Point,
    pub segments: Vec<Segment>,
}

/// Segment of a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Line(Point),
    Arc {
        to: Point,
        radius: f32,
        clockwise: bool,
    },
}

/// Returns true if the character is drawn as a shape.
pub fn is_builtin(ch: char) -> bool {
    matches!(ch as u32, 0x2500..=0x259F | 0x1FB00..=0x1FB3B | 0x1CD00..=0x1CDE5)
}

/// Returns the shape of the character, if it is drawn as a shape.
pub fn glyph(ch: char, cell: Cell) -> Option<Glyph> {
    let code = ch as u32;
    let glyph = match code {
        0x2500..=0x257F => box_drawing(code, cell),
        0x2580..=0x259F => block(code, cell),
        0x1FB00..=0x1FB3B => blocks(sextant(code - 0x1FB00), 2, 3, cell),
        0x1CD00..=0x1CDE5 => blocks(OCTANTS[(code - 0x1CD00) as usize], 2, 4, cell),
        _ => return None,
    };

    Some(glyph)
}

// ---

/// Returns the shape of a block element.
fn block(code: u32, cell: Cell) -> Glyph {
    let Cell {
        width: w,
        height: h,
    } = cell;

    let rect = match code {
        0x2580 => Rect::new(0.0, 0.0, w, h / 2.0),
        0x2581..=0x2588 => Rect::new(0.0, h - h * (code - 0x2580) as f32 / 8.0, w, h),
        0x2589..=0x258F => Rect::new(0.0, 0.0, w * (0x2590 - code) as f32 / 8.0, h),
        0x2590 => Rect::new(w / 2.0, 0.0, w, h),
        0x2591..=0x2593 => {
            return Glyph {
                rects: vec![Rect::new(0.0, 0.0, w, h)],
                opacity: (code - 0x2590) as f32 / 4.0,
                ..Default::default()
            };
        }
        0x2594 => Rect::new(0.0, 0.0, w, h / 8.0),
        0x2595 => Rect::new(w * 7.0 / 8.0, 0.0, w, h),
        _ => return blocks(QUADRANTS[(code - 0x2596) as usize], 2, 2, cell),
    };

    Glyph::filled(vec![rect])
}

/// Returns the shape of a character made of blocks of a `cols` by `rows` grid.
///
/// Bit `i` of `pattern` corresponds to the block at column `i % cols` and row `i / cols`.
fn blocks(pattern: u8, cols: u8, rows: u8, cell: Cell) -> Glyph {
    let x = |col: u8| cell.width * col as f32 / cols as f32;
    let y = |row: u8| cell.height * row as f32 / rows as f32;

    let rects = (0..cols * rows)
        .filter(|i| pattern & (1 << i) != 0)
        .map(|i| {
            let (col, row) = (i % cols, i / cols);
            Rect::new(x(col), y(row), x(col + 1), y(row + 1))
        })
        .collect();

    Glyph::filled(rects)
}

/// Quadrant patterns of U+2596..=U+259F.
const QUADRANTS: [u8; 10] = [
    0b0100, 0b1000, 0b0001, 0b1101, 0b1001, 0b0111, 0b1011, 0b0010, 0b0110, 0b1110,
];

/// Returns the pattern of a sextant by its index in U+1FB00..=U+1FB3B.
///
/// The range skips the empty and full patterns and the left and right halves,
/// which are available as other block elements.
fn sextant(index: u32) -> u8 {
    let mut pattern = index + 1;
    if pattern >= 0b010101 {
        pattern += 1;
    }
    if pattern >= 0b101010 {
        pattern += 1;
    }
    pattern as u8
}

/// Octant patterns of U+1CD00..=U+1CDE5.
///
/// The range lists all patterns in ascending order, except those available as other block elements.
const OCTANTS: [u8; 230] = {
    const EXCLUDED: [u8; 26] = [
        0, 1, 2, 3, 5, 10, 15, 20, 40, 63, 64, 80, 85, 90, 95, 128, 160, 165, 170, 175, 192, 240,
        245, 250, 252, 255,
    ];

    let mut table = [0; 230];
    let (mut pattern, mut i, mut j) = (0, 0, 0);
    while i < table.len() {
        if j < EXCLUDED.len() && EXCLUDED[j] == pattern {
            j += 1;
        } else {
            table[i] = pattern;
            i += 1;
        }
        pattern += 1;
    }
    table
};

// ---

/// Returns the shape of a box-drawing character.
fn box_drawing(code: u32, cell: Cell) -> Glyph {
    let arms = BOX_ARMS[(code - 0x2500) as usize];
    if arms != 0 {
        return lines(arms, cell);
    }

    match code {
        0x2504..=0x250B => {
            let i = code - 0x2504;
            dashes(i & 2 == 0, i & 1 != 0, 3 + i / 4, cell)
        }
        0x254C..=0x254F => {
            let i = code - 0x254C;
            dashes(i & 2 == 0, i & 1 != 0, 2, cell)
        }
        0x256D..=0x2570 => arc(code, cell),
        _ => diagonals(code, cell),
    }
}

/// Weights of the arms of box-drawing characters in U+2500..=U+257F.
///
/// Each entry packs the weights of the up, right, down and left arms, two bits each,
/// starting from the most significant bits: 0 is none, 1 is light, 2 is heavy, 3 is double.
/// Zero entries are characters without arms: dashed lines, arcs and diagonals.
#[rustfmt::skip]
const BOX_ARMS: [u8; 128] = [
    0x11, 0x22, 0x44, 0x88, 0x00, 0x00, 0x00, 0x00, // ─━│┃┄┅┆┇
    0x00, 0x00, 0x00, 0x00, 0x14, 0x24, 0x18, 0x28, // ┈┉┊┋┌┍┎┏
    0x05, 0x06, 0x09, 0x0A, 0x50, 0x60, 0x90, 0xA0, // ┐┑┒┓└┕┖┗
    0x41, 0x42, 0x81, 0x82, 0x54, 0x64, 0x94, 0x58, // ┘┙┚┛├┝┞┟
    0x98, 0xA4, 0x68, 0xA8, 0x45, 0x46, 0x85, 0x49, // ┠┡┢┣┤┥┦┧
    0x89, 0x86, 0x4A, 0x8A, 0x15, 0x16, 0x25, 0x26, // ┨┩┪┫┬┭┮┯
    0x19, 0x1A, 0x29, 0x2A, 0x51, 0x52, 0x61, 0x62, // ┰┱┲┳┴┵┶┷
    0x91, 0x92, 0xA1, 0xA2, 0x55, 0x56, 0x65, 0x66, // ┸┹┺┻┼┽┾┿
    0x95, 0x59, 0x99, 0x96, 0xA5, 0x5A, 0x69, 0xA6, // ╀╁╂╃╄╅╆╇
    0x6A, 0x9A, 0xA9, 0xAA, 0x00, 0x00, 0x00, 0x00, // ╈╉╊╋╌╍╎╏
    0x33, 0xCC, 0x34, 0x1C, 0x3C, 0x07, 0x0D, 0x0F, // ═║╒╓╔╕╖╗
    0x70, 0xD0, 0xF0, 0x43, 0xC1, 0xC3, 0x74, 0xDC, // ╘╙╚╛╜╝╞╟
    0xFC, 0x47, 0xCD, 0xCF, 0x37, 0x1D, 0x3F, 0x73, // ╠╡╢╣╤╥╦╧
    0xD1, 0xF3, 0x77, 0xDD, 0xFF, 0x00, 0x00, 0x00, // ╨╩╪╫╬╭╮╯
    0x00, 0x00, 0x00, 0x00, 0x01, 0x40, 0x10, 0x04, // ╰╱╲╳╴╵╶╷
    0x02, 0x80, 0x20, 0x08, 0x21, 0x48, 0x12, 0x84, // ╸╹╺╻╼╽╾╿
];

/// Weight of an arm of a box-drawing character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Weight {
    None,
    Light,
    Heavy,
    Double,
}

impl Weight {
    fn unpack(arms: u8, shift: u8) -> Self {
        match (arms >> shift) & 3 {
            0 => Self::None,
            1 => Self::Light,
            2 => Self::Heavy,
            _ => Self::Double,
        }
    }

    fn thickness(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Light | Self::Double => LIGHT,
            Self::Heavy => HEAVY,
        }
    }

    fn is_some(self) -> bool {
        self != Self::None
    }
}

/// Returns the shape of a box-drawing character made of straight arms going from the center
/// to the middle of the cell edges.
fn lines(arms: u8, cell: Cell) -> Glyph {
    let [up, right, down, left] = [6, 4, 2, 0].map(|shift| Weight::unpack(arms, shift));
    let (cx, cy) = (cell.width / 2.0, cell.height / 2.0);

    let mut rects = Vec::new();
    let mut horizontal = |arm, edge, opposite| {
        for (along, across) in arm_lines(arm, cx, edge, cy, [up, down], opposite) {
            rects.push(Rect::new(along.0, across.0, along.1, across.1));
        }
    };
    horizontal(right, cell.width, left);
    horizontal(left, 0.0, right);

    let mut vertical = |arm, edge, opposite| {
        for (along, across) in arm_lines(arm, cy, edge, cx, [left, right], opposite) {
            rects.push(Rect::new(across.0, along.0, across.1, along.1));
        }
    };
    vertical(down, cell.height, up);
    vertical(up, 0.0, down);

    Glyph::filled(rects)
}

/// Returns the strokes of an arm as pairs of intervals along and across the arm.
///
/// The arm goes from `center` to `edge` along its axis and is centered at `middle` across it.
/// The `sides` are the perpendicular arms at the lower and the higher coordinates across the arm.
fn arm_lines(
    arm: Weight,
    center: f32,
    edge: f32,
    middle: f32,
    sides: [Weight; 2],
    opposite: Weight,
) -> Vec<((f32, f32), (f32, f32))> {
    let direction = (edge - center).signum();
    let along = |offset: f32| {
        let start = center + direction * offset;
        (start.min(edge), start.max(edge))
    };

    match arm {
        Weight::None => Vec::new(),
        Weight::Light | Weight::Heavy => {
            // Offset towards the edge where the stroke starts, so that it joins the perpendicular arms.
            let offset = if sides.contains(&Weight::Double) {
                if opposite.is_some() {
                    0.0
                } else if sides.iter().all(|side| side.is_some()) {
                    DOUBLE
                } else {
                    -DOUBLE
                }
            } else {
                -sides.map(Weight::thickness).into_iter().fold(0.0, f32::max) / 2.0
            };
            let half = arm.thickness() / 2.0;
            vec![(along(offset), (middle - half, middle + half))]
        }
        Weight::Double => [(-1.0, sides[0], sides[1]), (1.0, sides[1], sides[0])]
            .into_iter()
            .map(|(sign, near, far)| {
                let gap = |side: Weight| if side == Weight::Double { DOUBLE } else { 0.0 };
                let offset = if near.is_some() {
                    gap(near) - LIGHT / 2.0
                } else if opposite.is_some() || !far.is_some() {
                    0.0
                } else {
                    -gap(far) - LIGHT / 2.0
                };
                let stroke = middle + sign * DOUBLE;
                (along(offset), (stroke - LIGHT / 2.0, stroke + LIGHT / 2.0))
            })
            .collect(),
    }
}

/// Returns the shape of a dashed line with the given number of dashes per cell.
fn dashes(horizontal: bool, heavy: bool, count: u32, cell: Cell) -> Glyph {
    let half = if heavy { HEAVY } else { LIGHT } / 2.0;
    let (length, middle) = if horizontal {
        (cell.width, cell.height / 2.0)
    } else {
        (cell.height, cell.width / 2.0)
    };

    let rects = (0..count)
        .map(|i| {
            let start = length * (4 * i + 1) as f32 / (4 * count) as f32;
            let end = length * (4 * i + 3) as f32 / (4 * count) as f32;
            if horizontal {
                Rect::new(start, middle - half, end, middle + half)
            } else {
                Rect::new(middle - half, start, middle + half, end)
            }
        })
        .collect();

    Glyph::filled(rects)
}

/// Returns the shape of an arc connecting the middles of two adjacent cell edges.
fn arc(code: u32, cell: Cell) -> Glyph {
    // Horizontal and vertical directions of the arc ends: ╭ ╮ ╯ ╰
    let (dx, dy) = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][(code - 0x256D) as usize];
    let (cx, cy) = (cell.width / 2.0, cell.height / 2.0);
    let half = LIGHT / 2.0;

    // The ends of the arc are straight lines reaching the cell edges, drawn as rectangles so that
    // they join the lines in adjacent cells without seams. The curve overlaps them within the cell.
    let overlap = half;
    let radius = cx.min(cy) - overlap;
    let (hx, vy) = (cx + dx * radius, cy + dy * radius);
    let (ex, ey) = (cx + dx * cx, cy + dy * cy);

    Glyph {
        rects: vec![
            Rect::new(hx.min(ex), cy - half, hx.max(ex), cy + half),
            Rect::new(cx - half, vy.min(ey), cx + half, vy.max(ey)),
        ],
        polygons: Vec::new(),
        curves: vec![Curve {
            start: (hx + dx * overlap, cy),
            segments: vec![
                Segment::Line((hx, cy)),
                Segment::Arc {
                    to: (cx, vy),
                    radius,
                    clockwise: dx * dy < 0.0,
                },
                Segment::Line((cx, vy + dy * overlap)),
            ],
        }],
        opacity: 1.0,
    }
}

/// Returns the shape of a diagonal line or a diagonal cross.
///
/// Each line runs from corner to corner and is cut by the cell edges, so that lines in
/// adjacent cells meet flush at the shared edge.
fn diagonals(code: u32, cell: Cell) -> Glyph {
    let Cell {
        width: w,
        height: h,
    } = cell;

    let rising = band(cell, (0.0, h), (w, 0.0), LIGHT);
    let falling = band(cell, (0.0, 0.0), (w, h), LIGHT);

    Glyph::polygons(match code {
        0x2571 => vec![rising],
        0x2572 => vec![falling],
        _ => vec![rising, falling],
    })
}

/// Returns the part of the cell covered by a straight line of the given thickness through two points.
fn band(cell: Cell, a: Point, b: Point, thickness: f32) -> Vec<Point> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let normal = (-dy / len, dx / len);
    let offset = normal.0 * a.0 + normal.1 * a.1;
    let half = thickness / 2.0;

    let cell = vec![
        (0.0, 0.0),
        (cell.width, 0.0),
        (cell.width, cell.height),
        (0.0, cell.height),
    ];
    let polygon = clip(&cell, normal, offset + half);
    clip(&polygon, (-normal.0, -normal.1), -offset + half)
}

/// Clips a convex polygon to the half-plane where `normal · p <= limit`.
fn clip(polygon: &[Point], normal: Point, limit: f32) -> Vec<Point> {
    let distance = |p: Point| normal.0 * p.0 + normal.1 * p.1 - limit;
    let mut result = Vec::with_capacity(polygon.len() + 1);
    for (i, &p) in polygon.iter().enumerate() {
        let q = polygon[(i + 1) % polygon.len()];
        let (dp, dq) = (distance(p), distance(q));
        if dp <= 0.0 {
            result.push(p);
        }
        if (dp < 0.0) != (dq < 0.0) && dp != 0.0 && dq != 0.0 {
            let t = dp / (dp - dq);
            result.push((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t));
        }
    }
    result
}

// ---

impl Glyph {
    fn filled(rects: Vec<Rect>) -> Self {
        Self {
            rects,
            polygons: Vec::new(),
            curves: Vec::new(),
            opacity: 1.0,
        }
    }

    fn polygons(polygons: Vec<Vec<Point>>) -> Self {
        Self {
            rects: Vec::new(),
            polygons,
            curves: Vec::new(),
            opacity: 1.0,
        }
    }
}

impl Rect {
    fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }
}

#[cfg(test)]
mod tests;
