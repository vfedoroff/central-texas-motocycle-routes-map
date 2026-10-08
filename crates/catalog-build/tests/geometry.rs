use catalog_build::geometry::simplify_overview;
use catalog_build::places_along_route::point_to_polyline_distance_m;
use catalog_model::Coordinate;

#[test]
fn test_two_point_input_unchanged() {
    let line: Vec<Coordinate> = vec![[-98.0, 30.0], [-98.1, 30.1]];
    let simplified = simplify_overview(&line, 25.0);
    assert_eq!(simplified, line);
}

#[test]
fn test_open_endpoints_unchanged() {
    let mut line: Vec<Coordinate> = Vec::new();
    for i in 0..50 {
        let t = i as f64 * 0.001;
        line.push([-98.0 + t, 30.0 + (t * 20.0).sin() * 0.0005]);
    }

    let simplified = simplify_overview(&line, 25.0);
    assert!(simplified.len() < line.len());
    assert_eq!(simplified.first().unwrap(), line.first().unwrap());
    assert_eq!(simplified.last().unwrap(), line.last().unwrap());
}

#[test]
fn test_exact_loop_closure_unchanged() {
    let p_start: Coordinate = [-98.0, 30.0];
    let mut loop_line: Vec<Coordinate> = Vec::new();
    loop_line.push(p_start);
    for i in 1..80 {
        let angle = (i as f64 / 80.0) * std::f64::consts::TAU;
        loop_line.push([-98.0 + angle.cos() * 0.01, 30.0 + angle.sin() * 0.01]);
    }
    loop_line.push(p_start); // exactly closed

    let simplified = simplify_overview(&loop_line, 25.0);
    assert!(simplified.len() < loop_line.len());
    assert!(simplified.len() >= 4);
    assert_eq!(simplified.first().unwrap(), &p_start);
    assert_eq!(simplified.last().unwrap(), &p_start);
}

#[test]
fn test_retained_coordinates_are_exact_clones() {
    // Retained coordinates must not be rounded or modified
    let p1: Coordinate = [-98.12345678, 30.87654321];
    let p2: Coordinate = [-98.12345600, 30.87654300]; // slight wiggle within 1m
    let p3: Coordinate = [-98.20000000, 30.90000000];

    let line = vec![p1, p2, p3];
    let simplified = simplify_overview(&line, 25.0);

    // p2 is pruned because it's within 1m of line(p1, p3)
    assert_eq!(simplified.len(), 2);
    assert_eq!(simplified[0], p1);
    assert_eq!(simplified[1], p3);
}

#[test]
fn test_maximum_error_within_tolerance() {
    let tolerance = 25.0;

    let mut line: Vec<Coordinate> = Vec::new();
    for i in 0..100 {
        let t = i as f64 * 0.0005;
        // Wavy curve with some high-frequency wiggles
        let lon = -98.0 + t;
        let lat = 30.0 + (t * 15.0).sin() * 0.001 + (t * 50.0).sin() * 0.0001;
        line.push([lon, lat]);
    }

    let simplified = simplify_overview(&line, tolerance);
    assert!(simplified.len() < line.len());

    // Every original point should be within tolerance_m of the simplified polyline
    for p in &line {
        let dist = point_to_polyline_distance_m(*p, &simplified);
        assert!(
            dist <= tolerance * 1.05, // allow 5% margin for discrete projection approximation
            "Point {:?} distance {:.2}m exceeds tolerance {:.2}m",
            p,
            dist,
            tolerance
        );
    }
}
