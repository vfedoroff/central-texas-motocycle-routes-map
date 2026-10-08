use catalog_model::{
    AuthoredPlace, AuthoredRoad, AuthoredRoute, AuthoredStop, PlaceCategory, RouteType, Surface,
    load_catalog, validate_catalog_with_options,
};
use tempfile::tempdir;

#[test]
fn test_documentation_route_example_deserializes() {
    let route_json = r##"{
      "schema_version": 1,
      "id": "bertram-rm-1174-rm-963-loop",
      "title": "Bertram, RM 1174 & RM 963 Loop",
      "summary": "TX 29 West -> Bertram -> RM 1174 North -> RM 963 East -> US 183 South.",
      "category": "Plains & Ranchlands",
      "color": "#27ae60",
      "distance_mi": 54.7,
      "waypoints": [
        "183 / TX 29 Crossing",
        "Bertram",
        "RM 1174",
        "RM 963",
        "US 183"
      ],
      "via": "TX 29 West -> Bertram -> RM 1174 North -> RM 963 East -> US 183 South",
      "geometry_path": "content/geometry/routes/bertram-rm-1174-rm-963-loop.geojson",
      "route_type": "loop",
      "tags": [
        "hill-country",
        "scenic",
        "sweepers"
      ],
      "sources": [
        {
          "title": "Texas Department of Transportation Highway Designation Files",
          "url": "https://www.txdot.gov/data-maps/highway-designation-files.html",
          "accessed_on": "2026-10-07"
        }
      ],
      "stops": [
        {
          "place_id": "bertram-historic-depot",
          "note": "Rest stop and photo opportunity"
        }
      ],
      "navigation_junctions": [
        {
          "node_id": 1001,
          "label": "TX 29 & RM 1174 Junction",
          "coordinates": [-98.0552, 30.7441]
        }
      ],
      "author_note": {
        "visit_status": "visited",
        "visited_on": "2026-10-07",
        "recommendation": "recommend",
        "text": "Smooth asphalt sweepers through rolling cedar savanna. Early mornings have low traffic.",
        "updated_on": "2026-10-07"
      }
    }"##;

    let route: AuthoredRoute =
        serde_json::from_str(route_json).expect("Documentation route example must deserialize");
    assert_eq!(route.id, "bertram-rm-1174-rm-963-loop");
    assert_eq!(route.distance_mi, 54.7);
    assert_eq!(route.stops.len(), 1);
    assert_eq!(route.stops[0].place_id, "bertram-historic-depot");
    assert_eq!(route.route_type, RouteType::Loop);
}

#[test]
fn test_documentation_place_example_deserializes() {
    let place_json = r##"{
      "schema_version": 1,
      "id": "bertram-historic-depot",
      "title": "Historic Bertram Depot",
      "summary": "1934 Southern Pacific train station depot located in downtown Bertram.",
      "place_category": "landmark",
      "coordinates": [-98.0556, 30.7436],
      "body_markdown": "Restored 1934 wooden depot on the Austin Steam Train Association line. Shaded benches and historical marker.",
      "related": [
        {
          "kind": "route",
          "id": "bertram-rm-1174-rm-963-loop"
        }
      ],
      "photos": [
        {
          "path": "content/photos/places/bertram-depot-front.jpg",
          "alt": "Front elevation of Bertram historic wooden train depot",
          "credit": "Vadym Fedorov",
          "rights": "CC-BY-4.0"
        }
      ],
      "sources": [
        {
          "title": "Austin Steam Train Association Historic Preservation",
          "url": "https://www.austinsteamtrain.org/history-bertram-depot",
          "accessed_on": "2026-10-07"
        }
      ],
      "author_note": {
        "visit_status": "visited",
        "visited_on": "2026-10-07",
        "recommendation": "recommend",
        "text": "Great turnaround spot with street parking adjacent to public restrooms.",
        "updated_on": "2026-10-07"
      },
      "google_place_id": "ChIJb8R_4NfBRIYRzY-9iN4c7k4"
    }"##;

    let place: AuthoredPlace =
        serde_json::from_str(place_json).expect("Documentation place example must deserialize");
    assert_eq!(place.id, "bertram-historic-depot");
    assert_eq!(place.coordinates, [-98.0556, 30.7436]);
    assert_eq!(place.place_category, PlaceCategory::Landmark);
    assert_eq!(place.photos.len(), 1);
}

#[test]
fn test_documentation_road_example_deserializes() {
    let road_json = r##"{
      "schema_version": 1,
      "id": "rm-1174-north-sweep",
      "title": "RM 1174 North",
      "summary": "14-mile sweeping two-lane hill country connector between Bertram and RM 963.",
      "geometry_path": "content/geometry/roads/rm-1174-north-sweep.geojson",
      "surface": "paved",
      "surface_evidence": [
        {
          "title": "TxDOT Pavement Management System Survey",
          "url": "https://www.txdot.gov/data-maps/pavement-condition.html",
          "accessed_on": "2026-10-07"
        }
      ],
      "sources": [
        {
          "title": "Texas FM / RM Highway Log",
          "url": "https://www.txdot.gov",
          "accessed_on": "2026-10-07"
        }
      ],
      "author_note": {
        "visit_status": "visited",
        "visited_on": "2026-10-07",
        "recommendation": "recommend",
        "text": "Excellent road surface with consistent radius curves and minimal blind driveways.",
        "updated_on": "2026-10-07"
      }
    }"##;

    let road: AuthoredRoad =
        serde_json::from_str(road_json).expect("Documentation road example must deserialize");
    assert_eq!(road.id, "rm-1174-north-sweep");
    assert_eq!(road.surface, Surface::Paved);
    assert_eq!(road.surface_evidence.len(), 1);
}

#[test]
fn test_poi_workflow_broken_reference_detected() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("content/routes")).unwrap();
    std::fs::create_dir_all(root.join("content/geometry/routes")).unwrap();
    std::fs::create_dir_all(root.join("content/places")).unwrap();

    let p1 = [-98.05, 30.74];
    let p2 = [-98.06, 30.75];

    let geojson = serde_json::json!({
        "type": "Feature",
        "properties": {},
        "geometry": {
            "type": "LineString",
            "coordinates": [p1, p2, p1]
        }
    });
    std::fs::write(
        root.join("content/geometry/routes/sample-loop.geojson"),
        serde_json::to_string(&geojson).unwrap(),
    )
    .unwrap();

    // Route referencing a non-existent POI "ghost-poi"
    let route = AuthoredRoute {
        schema_version: 1,
        id: "sample-loop".into(),
        title: "Sample Loop".into(),
        summary: "A test loop".into(),
        body_markdown: String::new(),
        tags: vec!["scenic".into()],
        sources: Vec::new(),
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        category: "Plains & Ranchlands".into(),
        color: "#27ae60".into(),
        distance_mi: 15.0,
        waypoints: vec!["Start".into(), "End".into()],
        via: "County Road 100".into(),
        geometry_path: "content/geometry/routes/sample-loop.geojson".into(),
        route_type: RouteType::Loop,
        navigation_junctions: Vec::new(),
        stops: vec![AuthoredStop {
            place_id: "ghost-poi".into(),
            note: Some("Ghost Stop".into()),
        }],
    };
    std::fs::write(
        root.join("content/routes/sample-loop.json"),
        serde_json::to_string_pretty(&route).unwrap(),
    )
    .unwrap();

    let catalog = load_catalog(root).expect("Catalog should load with single route");
    let diags = validate_catalog_with_options(&catalog, true);

    let has_broken_poi = diags.iter().any(|d| d.code == "invalid_stop_target");
    assert!(
        has_broken_poi,
        "Validation must catch non-existent place reference in route stops. Got diagnostics: {:?}",
        diags
    );

    // Now author the missing POI
    let place = AuthoredPlace {
        schema_version: 1,
        id: "ghost-poi".into(),
        title: "Ghost POI".into(),
        summary: "Now verified landmark".into(),
        body_markdown: String::new(),
        tags: Vec::new(),
        sources: Vec::new(),
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        place_category: PlaceCategory::Landmark,
        coordinates: [-98.055, 30.745],
        address: None,
        google_place_id: None,
    };
    std::fs::write(
        root.join("content/places/ghost-poi.json"),
        serde_json::to_string_pretty(&place).unwrap(),
    )
    .unwrap();

    let catalog_fixed = load_catalog(root).expect("Catalog should load with route and place");
    let diags_fixed = validate_catalog_with_options(&catalog_fixed, true);

    let still_broken = diags_fixed.iter().any(|d| d.code == "invalid_stop_target");
    assert!(
        !still_broken,
        "Once POI is authored, stop reference must validate cleanly. Got: {:?}",
        diags_fixed
    );
}
