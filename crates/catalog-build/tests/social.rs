use catalog_build::social::MapViewport;

#[test]
fn mercator_matches_known_locations_and_keeps_route_inside_map() {
    let viewport = MapViewport::for_route(&[[-98.0, 30.0], [-97.0, 31.0]]).unwrap();
    let origin = MapViewport::project([0.0, 0.0]);
    assert!(origin[0].abs() < 1e-8 && origin[1].abs() < 1e-8);
    let point = MapViewport::project([-98.0, 30.0]);
    assert!((point[0] + 10_909_310.097_7).abs() < 0.01);
    assert!((point[1] - 3_503_549.843_5).abs() < 0.01);
    for coord in [[-98.0, 30.0], [-97.0, 31.0]] {
        let [x, y] = viewport.pixel(coord);
        assert!((100.0..2300.0).contains(&x));
        assert!((100.0..740.0).contains(&y));
    }
}

#[test]
fn invalid_coordinates_do_not_create_a_map() {
    assert!(MapViewport::for_route(&[]).is_err());
    assert!(MapViewport::for_route(&[[f64::NAN, 30.0]]).is_err());
    assert!(MapViewport::for_route(&[[-98.0, 90.0]]).is_err());
}
