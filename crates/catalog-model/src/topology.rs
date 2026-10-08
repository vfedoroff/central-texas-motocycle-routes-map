use crate::Coordinate;

#[derive(Clone, Debug, PartialEq)]
pub struct Spur {
    pub start_index: usize,
    pub end_index: usize,
    pub length_m: f64,
    pub gap_m: f64,
}

pub fn dist_m(p1: Coordinate, p2: Coordinate) -> f64 {
    let dx = (p1[0] - p2[0]) * (p1[1].to_radians()).cos() * 111320.0;
    let dy = (p1[1] - p2[1]) * 110540.0;
    dx.hypot(dy)
}

pub fn detect_spurs(coords: &[Coordinate]) -> Vec<Spur> {
    detect_spurs_with_params(coords, 80.0, 5000.0, 40.0, 400)
}

pub fn detect_spurs_with_params(
    coords: &[Coordinate],
    min_track_len: f64,
    max_track_len: f64,
    max_gap: f64,
    max_lookahead: usize,
) -> Vec<Spur> {
    let n = coords.len();
    if n < 5 {
        return Vec::new();
    }
    let mut cum = Vec::with_capacity(n);
    cum.push(0.0);
    for i in 1..n {
        let prev = cum[i - 1];
        cum.push(prev + dist_m(coords[i - 1], coords[i]));
    }

    let mut spurs = Vec::new();
    let mut i = 0;
    while i < n.saturating_sub(4) {
        let mut best = None;
        let limit = (i + max_lookahead).min(n);
        for j in (i + 4)..limit {
            let t_len = cum[j] - cum[i];
            if t_len > max_track_len {
                break;
            }
            if t_len >= min_track_len {
                let s_len = dist_m(coords[i], coords[j]);
                if s_len <= max_gap {
                    best = Some(Spur {
                        start_index: i,
                        end_index: j,
                        length_m: t_len,
                        gap_m: s_len,
                    });
                    break;
                }
            }
        }
        if let Some(spur) = best {
            i = spur.end_index;
            spurs.push(spur);
        } else {
            i += 1;
        }
    }
    spurs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_line_has_no_spurs() {
        // Straight line 1 km long
        let line = vec![
            [-98.0, 30.0],
            [-98.0, 30.002],
            [-98.0, 30.004],
            [-98.0, 30.006],
            [-98.0, 30.008],
            [-98.0, 30.010],
        ];
        let spurs = detect_spurs(&line);
        assert!(spurs.is_empty());
    }

    #[test]
    fn out_and_back_spur_detected() {
        // A path that goes north, does a 200m spur out east and back to the same point, then continues north
        let coords = vec![
            [-98.000, 30.000],
            [-98.000, 30.001],
            [-98.000, 30.002], // index 2: spur start
            [-97.999, 30.002], // 100m east
            [-97.998, 30.002], // 200m east turnaround
            [-97.999, 30.002], // return
            [-98.000, 30.002], // index 6: spur end (gap ~0m, path length ~400m)
            [-98.000, 30.003],
            [-98.000, 30.004],
        ];
        let spurs = detect_spurs(&coords);
        assert_eq!(spurs.len(), 1);
        assert_eq!(spurs[0].start_index, 2);
        assert_eq!(spurs[0].end_index, 6);
        assert!(spurs[0].length_m >= 80.0);
        assert!(spurs[0].gap_m <= 40.0);
    }

    #[test]
    fn valid_large_loop_not_flagged_as_spur() {
        // A loop with track length > 5000m should not be flagged as a spur
        // 50km circuit with points spaced around 1km
        let mut loop_coords = Vec::new();
        let steps = 50;
        for i in 0..=steps {
            let angle = (i as f64 / steps as f64) * std::f64::consts::TAU;
            let lon = -98.0 + 0.1 * angle.cos();
            let lat = 30.0 + 0.1 * angle.sin();
            loop_coords.push([lon, lat]);
        }
        let spurs = detect_spurs(&loop_coords);
        assert!(
            spurs.is_empty(),
            "Valid loop should not trigger spurs: {:?}",
            spurs
        );
    }
}
