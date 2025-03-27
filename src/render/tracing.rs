use std::collections::{BTreeMap, VecDeque};

pub type Point = (i32, i32); // cell boundary coordinates.
pub type Contour = Vec<Point>;
pub type Path = Vec<Contour>;

pub struct Shape<K> {
    pub key: K,
    pub path: Path,
}

/// Entry point: Given grid dimensions and a closure to get group key of a cell,
/// returns a vector of shapes (each with the group key and its path) formed
/// by adjacent cells of the same group.
/// The closure should return:
///   - Some(key) for a key belonging to the group identified by the key.
///   - None for an empty cell.
pub fn trace<K, F>(cols: usize, rows: usize, group: F) -> Vec<Shape<K>>
where
    F: Fn(usize, usize) -> Option<K>,
    K: PartialEq,
{
    let clusters = find_clusters(cols, rows, &group);
    let mut result = Vec::new();
    for (key, cluster) in clusters {
        let mask = create_mask(&cluster, cols, rows);
        let contours = extract_contours(&mask);
        let oriented = reorient_contours(contours);
        let path = oriented.into_iter().map(optimize_contour).collect();
        result.push(Shape { key, path });
    }
    result
}

type Position = (usize, usize); // (x, y) in grid cell coordinates.

struct Mask {
    cols: usize,
    rows: usize,
    data: Vec<bool>,
}

impl Mask {
    fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            data: vec![false; cols * rows],
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        self.data[y * self.cols + x]
    }

    fn set(&mut self, x: usize, y: usize, value: bool) {
        self.data[y * self.cols + x] = value;
    }
}

/// Optimizes a closed contour by removing redundant collinear points,
/// but if the contour is exactly a rectangle (four corners), no point is removed.
/// The input contour is expected to be closed (first equals last). The output
/// is an open contour (no duplicate closing point); use "Z" in SVG.
fn optimize_contour(mut contour: Contour) -> Contour {
    // If the contour is closed, remove the duplicate closing point.
    if contour.len() >= 2 && contour.first() == contour.last() {
        contour.pop();
    }

    // If the resulting contour has exactly 4 points, assume it's a rectangle and keep it.
    if contour.len() == 4 {
        return contour;
    }

    let n = contour.len();
    if n < 3 {
        return contour;
    }

    let mut optimized = Vec::with_capacity(n);

    // Process the contour as circular.
    for i in 0..n {
        let prev = contour[(i + n - 1) % n];
        let curr = contour[i];
        let next = contour[(i + 1) % n];

        // Remove the current point if it is collinear with its neighbors
        // (only consider strictly horizontal or vertical collinearity).
        if (prev.0 == curr.0 && curr.0 == next.0) || (prev.1 == curr.1 && curr.1 == next.1) {
            continue;
        } else {
            optimized.push(curr);
        }
    }

    optimized
}

/// Finds clusters of adjacent cells in an abstract grid based on the provided
/// comparison function. The grid has dimensions (cols, rows).
/// The group function should return Some(key) if the cell at (x,y) is part of a cluster,
/// or None if a cell is empty. Cells that are empty
/// (i.e. where comparing a cell to itself returns None) are skipped.
/// For each cluster, the group key is saved along with the list of cells belonging to the cluster.
fn find_clusters<K, F>(cols: usize, rows: usize, group: &F) -> Vec<(K, Vec<Position>)>
where
    F: Fn(usize, usize) -> Option<K>,
    K: PartialEq,
{
    let mut visited = Mask::new(cols, rows);
    let mut clusters = Vec::new();

    for y in 0..rows {
        for x in 0..cols {
            if visited.get(x, y) {
                continue;
            }

            let Some(key) = group(x, y) else {
                visited.set(x, y, true);
                continue;
            };

            let mut cluster_points = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back((x, y));
            visited.set(x, y, true);

            while let Some((cx, cy)) = queue.pop_front() {
                cluster_points.push((cx, cy)); // store as (x,y)
                // Check 4-connected neighbors.
                for (dy, dx) in &[(0, 1), (1, 0), (0, -1), (-1, 0)] {
                    let ny = cy as isize + dy;
                    let nx = cx as isize + dx;
                    if ny >= 0 && ny < rows as isize && nx >= 0 && nx < cols as isize {
                        let nyu = ny as usize;
                        let nxu = nx as usize;
                        if !visited.get(nxu, nyu) {
                            // Only add if the neighbor is identical to the representative.
                            if group(nxu, nyu).as_ref() == Some(&key) {
                                visited.set(nxu, nyu, true);
                                queue.push_back((nxu, nyu));
                            }
                            // Otherwise, leave it unvisited.
                        }
                    }
                }
            }

            clusters.push((key, cluster_points));
        }
    }

    clusters
}

/// Given a cluster (list of cell positions) and grid dimensions,
/// creates a binary mask (with the same dimensions) where cells in the cluster are true.
fn create_mask(cluster: &[Position], cols: usize, rows: usize) -> Mask {
    let mut mask = Mask::new(cols, rows);
    cluster.iter().for_each(|&(x, y)| mask.set(x, y, true));
    mask
}

// --- Boundary extraction (integer version) ---

/// For each cell in the mask that is true, checks its four edges.
/// For each edge that borders an off cell (or lies on the outer border), a segment is added.
/// A cell at grid coordinate (x,y) covers the region from (x,y) (top‑left)
/// to (x+1,y+1) (bottom‑right).
fn extract_boundary_segments(mask: &Mask) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();

    for y in 0..mask.rows {
        for x in 0..mask.cols {
            if !mask.get(x, y) {
                continue;
            }

            let x = x as i32;
            let y = y as i32;

            // Top edge: from (x,y) to (x+1,y)
            if y == 0 || !mask.get(x as usize, y as usize - 1) {
                segments.push(((x, y), (x + 1, y)));
            }

            // Right edge: from (x+1,y) to (x+1,y+1)
            if x as usize == mask.cols - 1 || !mask.get(x as usize + 1, y as usize) {
                segments.push(((x + 1, y), (x + 1, y + 1)));
            }

            // Bottom edge: from (x+1,y+1) to (x,y+1)
            if y as usize == mask.rows - 1 || !mask.get(x as usize, y as usize + 1) {
                segments.push(((x + 1, y + 1), (x, y + 1)));
            }

            // Left edge: from (x,y+1) to (x,y)
            if x == 0 || !mask.get(x as usize - 1, y as usize) {
                segments.push(((x, y + 1), (x, y)));
            }
        }
    }
    segments
}

/// Groups the given line segments into simplified, closed contours.
/// Consecutive collinear points are merged so that unnecessary intermediate points are omitted.
fn group_segments_into_contours(segments: Vec<(Point, Point)>) -> Vec<Contour> {
    // Build a map from starting point to segments.
    let mut seg_map: BTreeMap<Point, Vec<(Point, Point)>> = BTreeMap::new();
    for seg in segments {
        seg_map.entry(seg.0).or_default().push(seg);
    }

    let mut contours = Vec::new();
    while let Some((&start, segs)) = seg_map.iter_mut().next() {
        if segs.is_empty() {
            seg_map.remove(&start);
            continue;
        }

        let mut contour = Vec::new();
        let mut current = start;
        contour.push(current);

        loop {
            let next_seg = {
                if let Some(vec) = seg_map.get_mut(&current) {
                    if !vec.is_empty() {
                        Some(vec.remove(0))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let Some((_, next)) = next_seg else {
                break;
            };

            // Merge collinear points.
            if contour.len() >= 2 {
                let a = contour[contour.len() - 2];
                let b = contour[contour.len() - 1];
                let c = next;
                let collinear = (a.0 == b.0 && b.0 == c.0) || (a.1 == b.1 && b.1 == c.1);
                if collinear {
                    contour.pop();
                }
            }
            contour.push(next);
            current = next;
            if current == start {
                break;
            }
        }

        contours.push(contour);
    }

    contours
}

/// Extracts the contours (outer boundary and holes) as closed paths in integer boundary coordinates.
fn extract_contours(mask: &Mask) -> Vec<Contour> {
    group_segments_into_contours(extract_boundary_segments(mask))
}

/// Computes twice the signed area of a closed contour (using integer arithmetic).
/// Positive area indicates clockwise orientation and negative indicates counterclockwise orientation.
fn signed_area(contour: &[Point]) -> i32 {
    let n = contour.len();
    if n < 2 {
        return 0;
    }

    let mut area2 = 0;

    for i in 0..(n - 1) {
        let (x1, y1) = contour[i];
        let (x2, y2) = contour[i + 1];
        area2 += x1 * y2 - y1 * x2;
    }

    area2
}

/// Adjusts the contours' orientations so that the outer contour is clockwise
/// and the holes are counterclockwise. (Assuming that a clockwise contour
/// gives a positive signed area in our coordinate system.)
fn reorient_contours(mut contours: Vec<Contour>) -> Vec<Contour> {
    if contours.is_empty() {
        return contours;
    }

    // Determine the outer contour (the one with largest absolute area).
    let mut outer_index = 0;
    let mut max_area = 0;
    for (i, contour) in contours.iter().enumerate() {
        let area2 = signed_area(contour).abs();
        if area2 > max_area {
            max_area = area2;
            outer_index = i;
        }
    }

    // Force the outer contour to be clockwise (positive area).
    if signed_area(&contours[outer_index]) < 0 {
        contours[outer_index].reverse();
    }

    // Force holes (all others) to be counterclockwise (negative area).
    for (i, contour) in contours.iter_mut().enumerate() {
        if i == outer_index {
            continue;
        }
        if signed_area(contour) > 0 {
            contour.reverse();
        }
    }

    contours
}
