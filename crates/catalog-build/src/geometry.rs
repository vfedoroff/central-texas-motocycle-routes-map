use catalog_model::Coordinate;

pub const DEFAULT_OVERVIEW_TOLERANCE_M: f64 = 25.0;

#[derive(Clone, Copy, Debug)]
struct MetricPoint {
    x: f64,
    y: f64,
}

fn project(coord: Coordinate, cos_lat: f64) -> MetricPoint {
    MetricPoint {
        x: coord[0] * cos_lat * 111320.0,
        y: coord[1] * 110540.0,
    }
}

fn perp_distance(p: MetricPoint, a: MetricPoint, b: MetricPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq <= 1e-10 {
        return (p.x - a.x).hypot(p.y - a.y);
    }

    let num = ((p.x - a.x) * dy - (p.y - a.y) * dx).abs();
    num / len_sq.sqrt()
}

fn douglas_peucker_indices(
    pts: &[MetricPoint],
    start: usize,
    end: usize,
    tolerance: f64,
    retained: &mut Vec<bool>,
) {
    if end <= start + 1 {
        return;
    }

    let a = pts[start];
    let b = pts[end];

    let mut max_dist = 0.0;
    let mut split_idx = start;

    for (i, p) in pts.iter().enumerate().take(end).skip(start + 1) {
        let d = perp_distance(*p, a, b);
        if d > max_dist {
            max_dist = d;
            split_idx = i;
        }
    }

    if max_dist > tolerance {
        retained[split_idx] = true;
        douglas_peucker_indices(pts, start, split_idx, tolerance, retained);
        douglas_peucker_indices(pts, split_idx, end, tolerance, retained);
    }
}

/// Simplify a polyline for display overview using Douglas–Peucker in a local metric projection.
///
/// Preserves exact start and end vertices (including exact loop closure).
/// Retained vertices copy the original authored coordinate values unchanged.
pub fn simplify_overview(coords: &[Coordinate], tolerance_m: f64) -> Vec<Coordinate> {
    if coords.len() <= 2 {
        return coords.to_vec();
    }

    let n = coords.len();
    let sum_lat: f64 = coords.iter().map(|c| c[1]).sum();
    let mean_lat = (sum_lat / n as f64).to_radians();
    let cos_lat = mean_lat.cos();

    let pts: Vec<MetricPoint> = coords.iter().map(|&c| project(c, cos_lat)).collect();
    let mut retained = vec![false; n];
    retained[0] = true;
    retained[n - 1] = true;

    // Check if loop closure (first == last)
    let is_loop = coords[0] == coords[n - 1];

    if is_loop && n > 3 {
        // Find the point furthest from the start point to split the loop into two arcs
        let mut max_d = 0.0;
        let mut mid = n / 2;
        let start_pt = pts[0];
        for (i, p) in pts.iter().enumerate().take(n - 1).skip(1) {
            let d = (p.x - start_pt.x).hypot(p.y - start_pt.y);
            if d > max_d {
                max_d = d;
                mid = i;
            }
        }
        retained[mid] = true;
        douglas_peucker_indices(&pts, 0, mid, tolerance_m, &mut retained);
        douglas_peucker_indices(&pts, mid, n - 1, tolerance_m, &mut retained);
    } else {
        douglas_peucker_indices(&pts, 0, n - 1, tolerance_m, &mut retained);
    }

    let mut result = Vec::new();
    for (i, &keep) in retained.iter().enumerate() {
        if keep {
            result.push(coords[i]);
        }
    }

    // Ensure loop closure is strictly exact
    if is_loop && result.len() >= 2 {
        let last_idx = result.len() - 1;
        result[last_idx] = result[0];
    }

    result
}
