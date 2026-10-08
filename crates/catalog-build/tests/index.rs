use catalog_build::index::build_index;
use catalog_model::{
    Catalog, CatalogObject, CatalogPayload, Coordinate, ObjectKey, ObjectKind, PlaceCategory,
    RouteType,
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
        body_markdown: "Secret long body text that must not appear in summary".into(),
        tags: vec!["overlook".into(), "view".into()],
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

fn make_route(
    id: &str,
    title: &str,
    via: &str,
    waypoints: Vec<&str>,
    coords: Vec<Coordinate>,
) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/routes/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: id.into(),
        },
        title: title.into(),
        summary: "A route summary".into(),
        body_markdown: "Secret long route body markdown that must not appear in summary".into(),
        tags: vec!["hill-country".into()],
        sources: Vec::new(),
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Route {
            category: "Twisties & Canyons".into(),
            color: "#e74c3c".into(),
            distance_mi: 42.5,
            waypoints: waypoints.into_iter().map(String::from).collect(),
            via: via.into(),
            geometry_path: format!("content/geometry/routes/{id}.geojson"),
            route_type: RouteType::Loop,
            navigation_junctions: Vec::new(),
            stops: Vec::new(),
        },
        geometry: Some(coords),
    }
}

#[test]
fn test_index_stable_ordering() {
    let p2 = make_place("b-place", "B Place", [-98.1, 30.1]);
    let p1 = make_place("a-place", "A Place", [-98.2, 30.2]);
    let r2 = make_route(
        "z-route",
        "Z Route",
        "Via Z",
        vec!["W2"],
        vec![[-98.0, 30.0], [-98.0, 30.1]],
    );
    let r1 = make_route(
        "a-route",
        "A Route",
        "Via A",
        vec!["W1"],
        vec![[-98.0, 30.0], [-98.0, 30.1]],
    );

    // Feed in jumbled order
    let catalog = Catalog {
        objects: vec![p2, r2, p1, r1],
    };

    let index = build_index(&catalog);
    assert_eq!(index.objects.len(), 4);

    // Expected order: Route (a-route, z-route), then Place (a-place, b-place)
    assert_eq!(index.objects[0].key.kind, ObjectKind::Route);
    assert_eq!(index.objects[0].key.id, "a-route");
    assert_eq!(index.objects[1].key.kind, ObjectKind::Route);
    assert_eq!(index.objects[1].key.id, "z-route");
    assert_eq!(index.objects[2].key.kind, ObjectKind::Place);
    assert_eq!(index.objects[2].key.id, "a-place");
    assert_eq!(index.objects[3].key.kind, ObjectKind::Place);
    assert_eq!(index.objects[3].key.id, "b-place");
}

#[test]
fn test_index_exact_urls_and_bounds() {
    let route = make_route(
        "test-loop",
        "Test Loop",
        "FM 1431 -> US 281",
        vec!["Marble Falls", "Burnet"],
        vec![[-98.25, 30.50], [-98.10, 30.65], [-98.20, 30.55]],
    );
    let place = make_place("test-viewpoint", "Test Viewpoint", [-98.15, 30.58]);

    let catalog = Catalog {
        objects: vec![route, place],
    };

    let index = build_index(&catalog);

    // Route summary check
    let r_sum = index
        .objects
        .iter()
        .find(|o| o.key.id == "test-loop")
        .unwrap();
    assert_eq!(r_sum.page_url, "/routes/test-loop/index.html");
    assert_eq!(r_sum.detail_url, "/data/routes/test-loop.json");
    assert_eq!(
        r_sum.overview_url.as_deref(),
        Some("/data/overview/routes/test-loop.geojson")
    );
    assert_eq!(
        r_sum.geometry_url.as_deref(),
        Some("/data/geometry/routes/test-loop.geojson")
    );
    assert_eq!(r_sum.point, None);
    assert_eq!(r_sum.bounds, [-98.25, 30.50, -98.10, 30.65]);

    // Searchable waypoints and via
    assert!(r_sum.search_text.contains("marble falls"));
    assert!(r_sum.search_text.contains("burnet"));
    assert!(r_sum.search_text.contains("fm 1431"));
    assert!(!r_sum.search_text.contains("secret long route body"));

    // Place summary check
    let p_sum = index
        .objects
        .iter()
        .find(|o| o.key.id == "test-viewpoint")
        .unwrap();
    assert_eq!(p_sum.page_url, "/places/test-viewpoint/index.html");
    assert_eq!(p_sum.detail_url, "/data/places/test-viewpoint.json");
    assert_eq!(p_sum.overview_url, None);
    assert_eq!(p_sum.geometry_url, None);
    assert_eq!(p_sum.point, Some([-98.15, 30.58]));
    // Zero-area bounds
    assert_eq!(p_sum.bounds, [-98.15, 30.58, -98.15, 30.58]);
    assert!(!p_sum.search_text.contains("secret long body text"));
}

#[test]
fn test_index_route_type_and_start_label_with_legacy_json_fallback() {
    use catalog_model::{CatalogSummary, NavigationJunction, ObjectDetail};

    let mut route_with_junction = make_route(
        "junction-route",
        "Junction Route",
        "Via X",
        vec!["W1"],
        vec![[-98.0, 30.0], [-98.0, 30.1]],
    );
    if let CatalogPayload::Route {
        ref mut navigation_junctions,
        ..
    } = route_with_junction.payload
    {
        navigation_junctions.push(NavigationJunction {
            node_id: 12345,
            label: "US 281 & FM 1431, Marble Falls".into(),
            coordinates: Some([-98.27, 30.57]),
        });
    }

    let route_without_junction = make_route(
        "no-junction-route",
        "No Junction Route",
        "Via Y",
        vec!["W2"],
        vec![[-98.0, 30.0], [-98.0, 30.1]],
    );

    let catalog = Catalog {
        objects: vec![route_with_junction, route_without_junction],
    };
    let index = build_index(&catalog);

    let with_j = index
        .objects
        .iter()
        .find(|o| o.key.id == "junction-route")
        .unwrap();
    assert_eq!(with_j.route_type, Some(RouteType::Loop));
    assert_eq!(
        with_j.start_label.as_deref(),
        Some("US 281 & FM 1431, Marble Falls")
    );

    let without_j = index
        .objects
        .iter()
        .find(|o| o.key.id == "no-junction-route")
        .unwrap();
    assert_eq!(without_j.route_type, Some(RouteType::Loop));
    assert_eq!(without_j.start_label, None);

    // Legacy JSON deserialization without route_type and start_label
    let legacy_summary_json = r#"{
        "key": { "kind": "route", "id": "legacy-route" },
        "title": "Legacy Route",
        "summary": "Summary",
        "tags": [],
        "search_text": "search",
        "bounds": [-98.0, 30.0, -98.0, 30.0],
        "page_url": "/routes/legacy-route/index.html",
        "detail_url": "/data/routes/legacy-route.json"
    }"#;
    let parsed_sum: CatalogSummary =
        serde_json::from_str(legacy_summary_json).expect("deserialize legacy summary");
    assert_eq!(parsed_sum.route_type, None);
    assert_eq!(parsed_sum.start_label, None);

    let legacy_detail_json = r#"{
        "schema_version": 1,
        "key": { "kind": "route", "id": "legacy-route" },
        "title": "Legacy Route",
        "summary": "Summary",
        "body_html": "<p>Body</p>",
        "bounds": [-98.0, 30.0, -98.0, 30.0]
    }"#;
    let parsed_detail: ObjectDetail =
        serde_json::from_str(legacy_detail_json).expect("deserialize legacy detail");
    assert_eq!(parsed_detail.start_label, None);
}
