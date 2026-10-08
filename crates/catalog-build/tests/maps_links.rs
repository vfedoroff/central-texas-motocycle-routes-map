use catalog_build::{build_stage_links, google_maps_links};
use catalog_model::{
    CatalogObject, CatalogPayload, NavigationJunction, NavigationMode, ObjectKey, ObjectKind,
    PlaceCategory, RouteType, Surface,
};
use url::Url;

fn make_place(id: &str, title: &str, coords: [f64; 2], place_id: Option<&str>) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/places/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Place,
            id: id.to_string(),
        },
        title: title.to_string(),
        summary: "A place for testing".to_string(),
        body_markdown: String::new(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Place {
            place_category: PlaceCategory::Viewpoint,
            coordinates: coords,
            address: None,
            google_place_id: place_id.map(String::from),
        },
        geometry: None,
    }
}

fn make_road(id: &str, title: &str, coords: Vec<[f64; 2]>) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/roads/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Road,
            id: id.to_string(),
        },
        title: title.to_string(),
        summary: "A road for testing".to_string(),
        body_markdown: String::new(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Road {
            geometry_path: format!("content/geometry/roads/{id}.geojson"),
            surface: Surface::Paved,
            surface_evidence: vec![],
        },
        geometry: Some(coords),
    }
}

fn make_route_with_junctions(
    id: &str,
    title: &str,
    junction_coords: Vec<[f64; 2]>,
) -> CatalogObject {
    let navigation_junctions = junction_coords
        .into_iter()
        .enumerate()
        .map(|(i, c)| NavigationJunction {
            node_id: 1000 + i as i64,
            label: format!("Junction {i}"),
            coordinates: Some(c),
        })
        .collect();

    CatalogObject {
        schema_version: 1,
        source_path: format!("content/routes/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: id.to_string(),
        },
        title: title.to_string(),
        summary: "A route for testing".to_string(),
        body_markdown: String::new(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Route {
            category: "Twisties & Canyons".to_string(),
            color: "#e74c3c".to_string(),
            distance_mi: 50.0,
            waypoints: vec!["Start".to_string(), "End".to_string()],
            via: "Test Via".to_string(),
            geometry_path: format!("content/geometry/routes/{id}.geojson"),
            route_type: RouteType::Loop,
            navigation_junctions,
            stops: vec![],
        },
        geometry: Some(vec![[-98.0, 30.0], [-98.1, 30.1]]),
    }
}

#[test]
fn test_place_navigation_links() {
    let place = make_place("lookout-point", "Scenic Lookout", [-98.123, 30.456], None);
    let links = google_maps_links(&place);

    assert_eq!(links.len(), 1);
    let link = &links[0];
    assert_eq!(link.mode, NavigationMode::Destination);
    assert_eq!(link.label, "Directions to Scenic Lookout");

    let parsed = Url::parse(&link.url).expect("valid URL");
    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.host_str(), Some("www.google.com"));
    assert_eq!(parsed.path(), "/maps/dir/");

    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(query.get("api"), Some(&"1".to_string()));
    assert_eq!(query.get("travelmode"), Some(&"driving".to_string()));
    assert_eq!(
        query.get("destination"),
        Some(&"30.456,-98.123".to_string())
    );
    assert!(!query.contains_key("destination_place_id"));
}

#[test]
fn test_place_with_google_place_id() {
    let place = make_place(
        "historic-bakery",
        "Historic Bakery",
        [-98.5, 30.2],
        Some("ChIJN1t_tDeuEmsRUsoyG83frY4"),
    );
    let links = google_maps_links(&place);

    assert_eq!(links.len(), 1);
    let link = &links[0];
    assert_eq!(link.mode, NavigationMode::Destination);

    let parsed = Url::parse(&link.url).expect("valid URL");
    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(
        query.get("destination_place_id"),
        Some(&"ChIJN1t_tDeuEmsRUsoyG83frY4".to_string())
    );
}

#[test]
fn test_road_without_junctions_directions_to_start() {
    let road = make_road("rm-2222", "RM 2222", vec![[-97.77, 30.36], [-97.80, 30.38]]);
    let links = google_maps_links(&road);

    assert_eq!(links.len(), 1);
    let link = &links[0];
    assert_eq!(link.mode, NavigationMode::Start);
    assert_eq!(link.label, "Directions to start of RM 2222");

    let parsed = Url::parse(&link.url).expect("valid URL");
    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(query.get("destination"), Some(&"30.36,-97.77".to_string()));
    assert_eq!(query.get("api"), Some(&"1".to_string()));
    assert_eq!(query.get("travelmode"), Some(&"driving".to_string()));
}

#[test]
fn test_route_staged_junctions_7_points() {
    // 7 points: [0..=4] (stage 1) then [4..=6] (stage 2)
    let points = vec![
        [-98.00, 30.00], // 0: Origin S1
        [-98.01, 30.01], // 1: WP1 S1
        [-98.02, 30.02], // 2: WP2 S1
        [-98.03, 30.03], // 3: WP3 S1
        [-98.04, 30.04], // 4: Dest S1 & Origin S2
        [-98.05, 30.05], // 5: WP1 S2
        [-98.06, 30.06], // 6: Dest S2
    ];

    let route = make_route_with_junctions("seven-point-loop", "Seven Point Loop", points);
    let links = google_maps_links(&route);

    assert_eq!(links.len(), 2);

    // Stage 1
    assert_eq!(links[0].label, "Stage 1");
    assert_eq!(links[0].mode, NavigationMode::Stage);
    let parsed1 = Url::parse(&links[0].url).expect("valid URL");
    let query1: std::collections::HashMap<_, _> = parsed1.query_pairs().into_owned().collect();
    assert_eq!(query1.get("origin"), Some(&"30,-98".to_string()));
    assert_eq!(query1.get("destination"), Some(&"30.04,-98.04".to_string()));
    assert_eq!(
        query1.get("waypoints"),
        Some(&"30.01,-98.01|30.02,-98.02|30.03,-98.03".to_string())
    );

    // Stage 2: starts at preceding endpoint (index 4)
    assert_eq!(links[1].label, "Stage 2");
    assert_eq!(links[1].mode, NavigationMode::Stage);
    let parsed2 = Url::parse(&links[1].url).expect("valid URL");
    let query2: std::collections::HashMap<_, _> = parsed2.query_pairs().into_owned().collect();
    assert_eq!(query2.get("origin"), Some(&"30.04,-98.04".to_string()));
    assert_eq!(query2.get("destination"), Some(&"30.06,-98.06".to_string()));
    assert_eq!(query2.get("waypoints"), Some(&"30.05,-98.05".to_string()));
}

#[test]
fn test_build_stage_links_short_points() {
    let points = vec![[-98.0, 30.0], [-98.1, 30.1]];
    let links = build_stage_links("Short Route", &points);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].label, "Stage 1");
    let parsed = Url::parse(&links[0].url).expect("valid URL");
    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(query.get("origin"), Some(&"30,-98".to_string()));
    assert_eq!(query.get("destination"), Some(&"30.1,-98.1".to_string()));
    assert!(!query.contains_key("waypoints"));
}

#[test]
fn test_url_length_limit_and_encoding() {
    let points = vec![
        [-98.0000001, 30.0000001],
        [-98.0000002, 30.0000002],
        [-98.0000003, 30.0000003],
    ];
    let links = build_stage_links("Encoding Test", &points);
    assert_eq!(links.len(), 1);

    // URL must be well under 2,048 chars
    assert!(links[0].url.len() <= 2048);

    // URL must be properly parsed
    assert!(Url::parse(&links[0].url).is_ok());
}

#[test]
fn test_missing_navigation_when_no_geometry() {
    let mut route = make_route_with_junctions("no-geom-route", "No Geom Route", vec![]);
    route.geometry = None;
    let links = google_maps_links(&route);
    assert!(links.is_empty());
}

#[test]
fn test_route_without_junctions_falls_back_to_start_link() {
    let mut route = make_route_with_junctions("fallback-route", "Fallback Route", vec![]);
    route.geometry = Some(vec![[-98.25, 30.15], [-98.30, 30.20]]);
    let links = google_maps_links(&route);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].mode, NavigationMode::Start);
    assert_eq!(links[0].label, "Directions to start of Fallback Route");

    let parsed = Url::parse(&links[0].url).expect("valid URL");
    let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
    assert_eq!(query.get("destination"), Some(&"30.15,-98.25".to_string()));
    assert_eq!(query.get("api"), Some(&"1".to_string()));
    assert_eq!(query.get("travelmode"), Some(&"driving".to_string()));
}

#[test]
fn test_staged_navigation_retains_numbered_wording_without_start_cta_replacement() {
    let points = vec![[-98.00, 30.00], [-98.01, 30.01], [-98.02, 30.02]];
    let route = make_route_with_junctions("staged-route", "Staged Route", points);
    let links = google_maps_links(&route);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].mode, NavigationMode::Stage);
    assert_eq!(links[0].label, "Stage 1");
}
