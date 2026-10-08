use catalog_roads::{
    AlignmentStatus, AuditConfig, Coordinate, compare_track, point_distance_m, resample_polyline,
};

#[test]
fn test_resample_polyline() {
    let line: Vec<Coordinate> = vec![[-98.0, 30.0], [-98.0, 30.01]];
    // 0.01 deg lat is ~1105 meters
    let resampled = resample_polyline(&line, 10.0);
    assert!(resampled.len() > 100);
    assert_eq!(resampled.first().unwrap(), &line[0]);
    assert_eq!(resampled.last().unwrap(), &line[1]);

    // Check adjacent distances
    for i in 0..resampled.len() - 1 {
        let d = point_distance_m(resampled[i], resampled[i + 1]);
        assert!(d <= 10.05); // within 10m step tolerance
    }
}

#[test]
fn test_follows_curve_passes() {
    let config = AuditConfig::default(); // max_deviation_m = 15.0

    // Road curve
    let mut path = Vec::new();
    let mut track = Vec::new();
    for i in 0..20 {
        let t = i as f64 * 0.0005;
        let lat = 30.0 + t;
        let lon = -98.0 + (t * 10.0).sin() * 0.001;
        path.push([lon, lat]);
        // Track follows within ~3 meters
        track.push([lon + 0.00002, lat]);
    }

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Pass);
    assert!(report.max_deviation_m.unwrap() < 5.0);
}

#[test]
fn test_shortcut_chord_through_bend_fails() {
    let config = AuditConfig::default(); // 15m tolerance

    // Path curves around a hill: U-shaped bend
    // (0,0) -> (100m, 100m) -> (200m, 0)
    let path = vec![
        [-98.000, 30.000],
        [-97.999, 30.001], // ~140m away
        [-97.998, 30.000],
    ];

    // Track cuts a straight chord directly from start to end, skipping the bend
    let track = vec![
        [-98.000, 30.000],
        [-97.998, 30.000], // Straight line chord through the bend
    ];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Fail);
    assert!(report.max_deviation_m.unwrap() > 50.0);
}

#[test]
fn test_swapped_traversal_order_fails() {
    let config = AuditConfig::default();

    // 1 km line
    let path = vec![[-98.00, 30.00], [-98.00, 30.01]];
    // Track travels in opposite direction
    let track = vec![[-98.00, 30.01], [-98.00, 30.00]];

    let report = compare_track(&track, &path, &config);
    // Swapped traversal cannot be monotonically coupled within 15m tolerance
    assert!(report.status != AlignmentStatus::Pass);
}

#[test]
fn test_missing_branch_fails() {
    let config = AuditConfig::default();

    // Path includes an out-and-back scenic spur / branch
    let path = vec![
        [-98.00, 30.00],
        [-98.00, 30.005],
        [-97.99, 30.005], // 1km side branch
        [-98.00, 30.005],
        [-98.00, 30.01],
    ];

    // Track skips the branch
    let track = vec![[-98.00, 30.00], [-98.00, 30.01]];

    let report = compare_track(&track, &path, &config);
    assert!(report.status != AlignmentStatus::Pass);
}

#[test]
fn test_out_and_back_deviation_fails() {
    let config = AuditConfig::default();

    let path = vec![[-98.00, 30.00], [-98.00, 30.01]];

    // Track deviates 100m to the side and comes back
    let track = vec![
        [-98.000, 30.000],
        [-98.000, 30.004],
        [-97.999, 30.005], // ~100m east
        [-98.000, 30.006],
        [-98.000, 30.010],
    ];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Fail);
    assert!(report.max_deviation_m.unwrap() > 50.0);
}

#[test]
fn test_parallel_route_ambiguity_fails() {
    let config = AuditConfig::default(); // 15m max deviation

    // Path on Highway Frontage Road A
    let path = vec![[-98.0000, 30.00], [-98.0000, 30.01]];
    // Track on Main Highway B parallel, ~30 meters west (-0.0003 lon ~ 29m)
    let track = vec![[-98.0003, 30.00], [-98.0003, 30.01]];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Fail);
    assert!(report.max_deviation_m.unwrap() > 20.0);
}

#[test]
fn test_threshold_below_above_15m() {
    let config = AuditConfig {
        max_deviation_m: 15.0,
        comparison_step_m: 5.0,
        closure_tolerance_m: 0.0,
        arc_band_m: 500.0,
        require_paved: true,
        schema_version: 1,
    };

    let path = vec![[-98.00, 30.00], [-98.00, 30.005]];

    // Track 10m away (~0.0001 lon is ~9.6m at lat 30)
    let track_pass = vec![[-98.0001, 30.00], [-98.0001, 30.005]];
    let report_pass = compare_track(&track_pass, &path, &config);
    assert_eq!(report_pass.status, AlignmentStatus::Pass);
    assert!(report_pass.max_deviation_m.unwrap() < 15.0);

    // Track 25m away (~0.00026 lon is ~25m)
    let track_fail = vec![[-98.00026, 30.00], [-98.00026, 30.005]];
    let report_fail = compare_track(&track_fail, &path, &config);
    assert_eq!(report_fail.status, AlignmentStatus::Fail);
    assert!(report_fail.max_deviation_m.unwrap() > 15.0);
}

#[test]
fn test_exact_loop_closure() {
    let config = AuditConfig::default();

    // Loop triangle: A -> B -> C -> A
    let a: Coordinate = [-98.00, 30.00];
    let b: Coordinate = [-98.00, 30.01];
    let c: Coordinate = [-97.99, 30.00];

    let path = vec![a, b, c, a];
    let track = vec![a, b, c, a];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Pass);
    assert!(report.max_deviation_m.unwrap() < 0.1);
}

#[test]
fn test_endpoints_on_road_interior_leaves_road() {
    let config = AuditConfig::default();

    // Start and end are exactly on road, but middle leaves road by 200m
    let path = vec![[-98.00, 30.00], [-98.00, 30.005], [-98.00, 30.01]];
    let track = vec![
        [-98.00, 30.00],
        [-97.998, 30.005], // ~190m east
        [-98.00, 30.01],
    ];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::Fail);
    assert!(report.max_deviation_m.unwrap() > 100.0);
}

#[test]
fn test_band_exhaustion_reports_review_required() {
    let config = AuditConfig {
        arc_band_m: 50.0, // very tight 50m band
        ..Default::default()
    };

    // Track starts 100m ahead along arc-length
    let path = vec![[-98.00, 30.00], [-98.00, 30.01]];
    let track = vec![
        [-98.00, 30.002], // ~220m ahead
        [-98.00, 30.01],
    ];

    let report = compare_track(&track, &path, &config);
    assert_eq!(report.status, AlignmentStatus::ReviewRequired);
    assert!(report.band_exhausted);
}
