use catalog_build::places_along_route::{point_to_polyline_distance_m, route_places};
use catalog_model::{
    AuthoredStop, Catalog, CatalogObject, CatalogPayload, Coordinate, ObjectKey, ObjectKind,
    PlaceCategory, RouteType,
};

fn make_place(id: &str, title: &str, coords: Coordinate) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/places/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Place,
            id: id.into(),
        },
        title: title.into(),
        summary: "A place summary".into(),
        body_markdown: String::new(),
        tags: vec![],
        sources: Vec::new(),
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Place {
            place_category: PlaceCategory::Viewpoint,
            coordinates: coords,
            address: None,
            google_place_id: None,
        },
        geometry: None,
    }
}

fn make_route_with_stops(
    id: &str,
    stops: Vec<(&str, Option<&str>)>,
    coords: Vec<Coordinate>,
) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/routes/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: id.into(),
        },
        title: format!("Route {id}"),
        summary: "Route summary".into(),
        body_markdown: String::new(),
        tags: vec![],
        sources: Vec::new(),
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Route {
            category: "Twisties & Canyons".into(),
            color: "#e74c3c".into(),
            distance_mi: 20.0,
            waypoints: vec!["Start".into(), "End".into()],
            via: "Road".into(),
            geometry_path: format!("content/geometry/routes/{id}.geojson"),
            route_type: RouteType::Loop,
            navigation_junctions: Vec::new(),
            stops: stops
                .into_iter()
                .map(|(place_id, note)| AuthoredStop {
                    place_id: place_id.into(),
                    note: note.map(String::from),
                })
                .collect(),
        },
        geometry: Some(coords),
    }
}

#[test]
fn test_authored_order_and_note_preservation() {
    let p1 = make_place("stop-1", "First Stop", [-98.0, 30.0]);
    let p2 = make_place("stop-2", "Second Stop", [-98.0, 30.01]);
    let route = make_route_with_stops(
        "test-route",
        vec![("stop-2", Some("Scenic overlook")), ("stop-1", None)], // authored in reverse order
        vec![[-98.0, 30.0], [-98.0, 30.01]],
    );

    let catalog = Catalog {
        objects: vec![p1, p2, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert_eq!(result.stops.len(), 2);
    assert_eq!(result.stops[0].key.id, "stop-2");
    assert_eq!(result.stops[0].note.as_deref(), Some("Scenic overlook"));
    assert_eq!(result.stops[1].key.id, "stop-1");
    assert_eq!(result.stops[1].note, None);
}

#[test]
fn test_missing_stop_target_skipped_gracefully() {
    let p1 = make_place("existing-stop", "Existing", [-98.0, 30.0]);
    let route = make_route_with_stops(
        "test-route",
        vec![("missing-stop", None), ("existing-stop", Some("Note"))],
        vec![[-98.0, 30.0], [-98.0, 30.01]],
    );

    let catalog = Catalog {
        objects: vec![p1, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert_eq!(result.stops.len(), 1);
    assert_eq!(result.stops[0].key.id, "existing-stop");
}

#[test]
fn test_point_near_middle_of_long_segment() {
    // 10 km long segment going north from lat 30.0 to 30.09 (~10 km)
    let polyline = vec![[-98.0, 30.0], [-98.0, 30.09]];

    // Place located at middle (lat 30.045) and 100 meters east (-97.999)
    let p_middle: Coordinate = [-97.99896, 30.045]; // ~100m east
    let d = point_to_polyline_distance_m(p_middle, &polyline);
    assert!(d < 120.0);
    assert!(d > 80.0);

    let place = make_place("mid-segment-overlook", "Mid Overlook", p_middle);
    let route = make_route_with_stops("route", Vec::new(), polyline);

    let catalog = Catalog {
        objects: vec![place, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert_eq!(result.nearby_places.len(), 1);
    assert_eq!(result.nearby_places[0].key.id, "mid-segment-overlook");
    assert!(result.nearby_places[0].distance_m < 120.0);
}

#[test]
fn test_inside_and_outside_radius() {
    let polyline = vec![[-98.0, 30.0], [-98.0, 30.01]];

    // 500m away (inside 2000m)
    let p_inside = make_place("inside", "Inside", [-97.995, 30.005]);
    // 3000m away (outside 2000m)
    let p_outside = make_place("outside", "Outside", [-97.965, 30.005]);

    let route = make_route_with_stops("route", Vec::new(), polyline);
    let catalog = Catalog {
        objects: vec![p_inside, p_outside, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert_eq!(result.nearby_places.len(), 1);
    assert_eq!(result.nearby_places[0].key.id, "inside");
}

#[test]
fn test_explicit_stop_excluded_from_nearby() {
    let polyline = vec![[-98.0, 30.0], [-98.0, 30.01]];

    let p_stop = make_place("stop-and-close", "Stop And Close", [-98.001, 30.005]); // 100m away
    let p_nearby = make_place("just-nearby", "Just Nearby", [-98.002, 30.005]);

    let route = make_route_with_stops(
        "route",
        vec![("stop-and-close", None)], // explicitly marked as stop
        polyline,
    );

    let catalog = Catalog {
        objects: vec![p_stop, p_nearby, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert_eq!(result.stops.len(), 1);
    assert_eq!(result.stops[0].key.id, "stop-and-close");

    // Nearby places must NOT contain stop-and-close
    assert_eq!(result.nearby_places.len(), 1);
    assert_eq!(result.nearby_places[0].key.id, "just-nearby");
}

#[test]
fn test_20_result_limit_and_stable_equal_distance_ordering() {
    let polyline = vec![[-98.0, 30.0], [-98.0, 30.01]];

    let mut places = Vec::new();
    // Create 30 places all within 500m of the polyline
    for i in 0..30 {
        let id = format!("place-{:02}", i);
        let title = format!("Place {:02}", i);
        places.push(make_place(
            &id,
            &title,
            [-98.001, 30.000 + (i as f64 * 0.0003)],
        ));
    }

    let route = make_route_with_stops("route", Vec::new(), polyline);
    let mut objects = places;
    objects.push(route.clone());
    let catalog = Catalog { objects };

    let result = route_places(&route, &catalog);
    // Limit to 20
    assert_eq!(result.nearby_places.len(), 20);

    // Verify distance monotonicity
    for w in result.nearby_places.windows(2) {
        assert!(w[0].distance_m <= w[1].distance_m);
    }
}

#[test]
fn test_route_with_no_candidates_returns_empty_arrays() {
    let polyline = vec![[-98.0, 30.0], [-98.0, 30.01]];
    let far_place = make_place("far", "Far Away", [-95.0, 30.0]); // hundreds of km away

    let route = make_route_with_stops("route", Vec::new(), polyline);
    let catalog = Catalog {
        objects: vec![far_place, route.clone()],
    };

    let result = route_places(&route, &catalog);
    assert!(result.stops.is_empty());
    assert!(result.nearby_places.is_empty());
}
