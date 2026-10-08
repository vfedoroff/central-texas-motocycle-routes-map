use crate::graph::haversine_distance_m;
use serde::{Deserialize, Serialize};

pub type Coordinate = [f64; 2];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditConfig {
    pub schema_version: u32,
    pub max_deviation_m: f64,
    pub comparison_step_m: f64,
    pub closure_tolerance_m: f64,
    pub arc_band_m: f64,
    pub require_paved: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            max_deviation_m: 15.0,
            comparison_step_m: 5.0,
            closure_tolerance_m: 0.0,
            arc_band_m: 500.0,
            require_paved: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentStatus {
    Pass,
    Fail,
    ReviewRequired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlignmentReport {
    pub status: AlignmentStatus,
    pub max_deviation_m: Option<f64>,
    pub cells_evaluated: usize,
    pub band_exhausted: bool,
    pub message: String,
}

/// Calculate local metric distance between two points in meters using haversine.
pub fn point_distance_m(p1: Coordinate, p2: Coordinate) -> f64 {
    haversine_distance_m(p1, p2)
}

/// Resample a polyline so that no adjacent vertices are farther apart than `step_m`.
/// All original vertices and endpoints are preserved.
pub fn resample_polyline(coords: &[Coordinate], step_m: f64) -> Vec<Coordinate> {
    if coords.len() < 2 {
        return coords.to_vec();
    }
    let mut resampled = Vec::with_capacity(coords.len() * 2);
    resampled.push(coords[0]);

    for i in 0..coords.len() - 1 {
        let p1 = coords[i];
        let p2 = coords[i + 1];
        let seg_len = point_distance_m(p1, p2);

        if seg_len > step_m && step_m > 0.0 {
            let num_steps = (seg_len / step_m).ceil() as usize;
            for s in 1..num_steps {
                let t = s as f64 / num_steps as f64;
                let lon = p1[0] + t * (p2[0] - p1[0]);
                let lat = p1[1] + t * (p2[1] - p1[1]);
                resampled.push([lon, lat]);
            }
        }
        resampled.push(p2);
    }

    resampled
}

/// Compute cumulative arc lengths along a polyline.
fn cumulative_arc_lengths(coords: &[Coordinate]) -> Vec<f64> {
    let mut lengths = Vec::with_capacity(coords.len());
    lengths.push(0.0);
    for i in 1..coords.len() {
        let prev = lengths[i - 1];
        lengths.push(prev + point_distance_m(coords[i - 1], coords[i]));
    }
    lengths
}

const MAX_DP_CELLS: usize = 20_000_000;

/// Compare an authored track polyline to a verified path polyline using band-constrained discrete Fréchet minimax DP.
///
/// Both polylines are resampled at `config.comparison_step_m` (default 5m).
/// DP memory is bounded to two sparse/compact rows, only evaluating index pairs with cumulative arc-length difference <= `config.arc_band_m` (default 500m).
/// If cells evaluated exceed 20,000,000 or the band is exhausted, returns `ReviewRequired`.
pub fn compare_track(
    track: &[Coordinate],
    path_line: &[Coordinate],
    config: &AuditConfig,
) -> AlignmentReport {
    if track.len() < 2 || path_line.len() < 2 {
        return AlignmentReport {
            status: AlignmentStatus::Fail,
            max_deviation_m: None,
            cells_evaluated: 0,
            band_exhausted: false,
            message: "Track or path line has fewer than 2 points".into(),
        };
    }

    let track_resampled = resample_polyline(track, config.comparison_step_m);
    let path_resampled = resample_polyline(path_line, config.comparison_step_m);

    let n = track_resampled.len();
    let m = path_resampled.len();

    let arc_track = cumulative_arc_lengths(&track_resampled);
    let arc_path = cumulative_arc_lengths(&path_resampled);

    let mut prev_row = vec![f64::INFINITY; m];
    let mut curr_row = vec![f64::INFINITY; m];
    let mut cells_evaluated = 0;

    // Row 0
    let mut row_has_reachable = false;
    for j in 0..m {
        if (arc_track[0] - arc_path[j]).abs() > config.arc_band_m {
            continue;
        }
        cells_evaluated += 1;
        let d = point_distance_m(track_resampled[0], path_resampled[j]);
        if j == 0 {
            curr_row[0] = d;
            row_has_reachable = true;
        } else if curr_row[j - 1].is_finite() {
            curr_row[j] = d.max(curr_row[j - 1]);
            row_has_reachable = true;
        }
    }

    if !row_has_reachable {
        return AlignmentReport {
            status: AlignmentStatus::ReviewRequired,
            max_deviation_m: None,
            cells_evaluated,
            band_exhausted: true,
            message: "Band exhausted at initial vertex (start points too far apart)".into(),
        };
    }

    prev_row.copy_from_slice(&curr_row);

    // Rows 1..n
    for i in 1..n {
        curr_row.fill(f64::INFINITY);
        let mut curr_has_reachable = false;

        for j in 0..m {
            if (arc_track[i] - arc_path[j]).abs() > config.arc_band_m {
                continue;
            }
            cells_evaluated += 1;
            if cells_evaluated > MAX_DP_CELLS {
                return AlignmentReport {
                    status: AlignmentStatus::ReviewRequired,
                    max_deviation_m: None,
                    cells_evaluated,
                    band_exhausted: false,
                    message: "Deterministic limit of 20,000,000 DP cells exceeded".into(),
                };
            }

            let d = point_distance_m(track_resampled[i], path_resampled[j]);

            // Predecessors: (i-1, j), (i-1, j-1), (i, j-1)
            let mut min_pred = prev_row[j];
            if j > 0 {
                min_pred = min_pred.min(prev_row[j - 1]).min(curr_row[j - 1]);
            }

            if min_pred.is_finite() {
                curr_row[j] = d.max(min_pred);
                curr_has_reachable = true;
            }
        }

        if !curr_has_reachable {
            return AlignmentReport {
                status: AlignmentStatus::ReviewRequired,
                max_deviation_m: None,
                cells_evaluated,
                band_exhausted: true,
                message: format!("Band exhausted at track index {} (coupling lost)", i),
            };
        }

        prev_row.copy_from_slice(&curr_row);
    }

    let final_deviation = prev_row[m - 1];

    if final_deviation.is_infinite() {
        AlignmentReport {
            status: AlignmentStatus::ReviewRequired,
            max_deviation_m: None,
            cells_evaluated,
            band_exhausted: true,
            message: "Coupling path did not reach destination within arc-length band".into(),
        }
    } else if final_deviation <= config.max_deviation_m {
        AlignmentReport {
            status: AlignmentStatus::Pass,
            max_deviation_m: Some(final_deviation),
            cells_evaluated,
            band_exhausted: false,
            message: format!(
                "Track aligns with path within tolerance ({:.2}m <= {:.2}m)",
                final_deviation, config.max_deviation_m
            ),
        }
    } else {
        AlignmentReport {
            status: AlignmentStatus::Fail,
            max_deviation_m: Some(final_deviation),
            cells_evaluated,
            band_exhausted: false,
            message: format!(
                "Track deviation {:.2}m exceeds tolerance {:.2}m",
                final_deviation, config.max_deviation_m
            ),
        }
    }
}
