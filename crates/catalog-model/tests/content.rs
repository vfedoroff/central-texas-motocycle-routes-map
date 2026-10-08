use catalog_model::{
    AuthorNote, AuthoredStop, Catalog, CatalogObject, CatalogPayload, Coordinate, ObjectKey,
    ObjectKind, PlaceCategory, Recommendation, RouteType, Source, VisitStatus, validate_catalog,
    validate_catalog_with_options,
};

fn sample_place(id: &str) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/places/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Place,
            id: id.into(),
        },
        title: format!("Place {id}"),
        summary: "A place summary".into(),
        body_markdown: String::new(),
        tags: vec!["scenic".into()],
        sources: vec![Source {
            title: "Source".into(),
            url: "https://example.com".into(),
            accessed_on: "2026-10-07".into(),
        }],
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Place {
            place_category: PlaceCategory::Viewpoint,
            coordinates: [-98.0, 30.0],
            address: None,
            google_place_id: None,
        },
        geometry: None,
    }
}

fn sample_route(id: &str, route_type: RouteType, coords: Vec<Coordinate>) -> CatalogObject {
    CatalogObject {
        schema_version: 1,
        source_path: format!("content/routes/{id}.json"),
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: id.into(),
        },
        title: format!("Route {id}"),
        summary: "A route summary".into(),
        body_markdown: String::new(),
        tags: vec!["hill-country".into()],
        sources: vec![Source {
            title: "Source".into(),
            url: "https://example.com/route".into(),
            accessed_on: "2026-10-07".into(),
        }],
        photos: Vec::new(),
        related: Vec::new(),
        updated_on: None,
        author_note: None,
        payload: CatalogPayload::Route {
            category: "Twisties & Canyons".into(),
            color: "#e74c3c".into(),
            distance_mi: 50.0,
            waypoints: vec!["Point A".into(), "Point B".into()],
            via: "RM 1431".into(),
            geometry_path: format!("content/geometry/routes/{id}.geojson"),
            route_type,
            navigation_junctions: Vec::new(),
            stops: Vec::new(),
        },
        geometry: Some(coords),
    }
}

#[test]
fn test_valid_catalog_passes() {
    let place = sample_place("enchanted-rock");
    let mut route = sample_route(
        "willow-city-loop",
        RouteType::Loop,
        vec![[-98.0, 30.0], [-98.05, 30.05], [-98.0, 30.0]],
    );
    route.related.push(place.key.clone());
    if let CatalogPayload::Route { stops, .. } = &mut route.payload {
        stops.push(AuthoredStop {
            place_id: "enchanted-rock".into(),
            note: None,
        });
    }

    let catalog = Catalog {
        objects: vec![place, route],
    };
    let diags = validate_catalog(&catalog);
    assert!(
        diags.is_empty(),
        "Expected no diagnostics, got: {:?}",
        diags
    );
}

#[test]
fn test_broken_relation_detected() {
    let mut route = sample_route(
        "lime-creek-road",
        RouteType::Corridor,
        vec![[-98.0, 30.0], [-98.1, 30.1]],
    );
    route.related.push(ObjectKey {
        kind: ObjectKind::Place,
        id: "non-existent-place".into(),
    });

    let catalog = Catalog {
        objects: vec![route],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "broken_relation"));
}

#[test]
fn test_invalid_stop_target_detected() {
    let mut route = sample_route(
        "lime-creek-road",
        RouteType::Corridor,
        vec![[-98.0, 30.0], [-98.1, 30.1]],
    );
    if let CatalogPayload::Route { stops, .. } = &mut route.payload {
        stops.push(AuthoredStop {
            place_id: "missing-stop-place".into(),
            note: None,
        });
    }

    let catalog = Catalog {
        objects: vec![route],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "invalid_stop_target"));
}

#[test]
fn test_filename_id_mismatch() {
    let mut place = sample_place("real-id");
    place.source_path = "content/places/different-filename.json".into();

    let catalog = Catalog {
        objects: vec![place],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "filename_id_mismatch"));
}

#[test]
fn test_duplicate_key_detected() {
    let place1 = sample_place("dupe-place");
    let place2 = sample_place("dupe-place");

    let catalog = Catalog {
        objects: vec![place1, place2],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "duplicate_key"));
}

#[test]
fn test_duplicate_tags_detected() {
    let mut place = sample_place("place-with-dupe-tags");
    place.tags = vec!["viewpoint".into(), "scenic".into(), "viewpoint".into()];

    let catalog = Catalog {
        objects: vec![place],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "duplicate_tag"));
}

#[test]
fn test_recommendation_without_visit_detected() {
    let mut place = sample_place("unvisited-recommendation");
    place.author_note = Some(AuthorNote {
        visit_status: VisitStatus::Planned,
        visited_on: None,
        recommendation: Some(Recommendation::Recommend),
        text: "Looks amazing!".into(),
        updated_on: "2026-10-07".into(),
    });

    let catalog = Catalog {
        objects: vec![place],
    };
    let diags = validate_catalog(&catalog);
    assert!(
        diags
            .iter()
            .any(|d| d.code == "recommendation_without_visit")
    );
}

#[test]
fn test_invalid_source_url_detected() {
    let mut place = sample_place("bad-source-url");
    place.sources = vec![Source {
        title: "Bad source".into(),
        url: "ftp://invalid-protocol.com/file".into(),
        accessed_on: "2026-10-07".into(),
    }];

    let catalog = Catalog {
        objects: vec![place],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "invalid_source_url"));
}

#[test]
fn test_path_traversal_in_photos_detected() {
    let mut place = sample_place("escaping-photo");
    place.photos = vec![catalog_model::Photo {
        path: "../../../etc/passwd".into(),
        alt: "escape".into(),
        credit: "none".into(),
        rights: "none".into(),
    }];

    let catalog = Catalog {
        objects: vec![place],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "path_traversal"));
}

#[test]
fn test_loop_not_closed_detected() {
    let route = sample_route(
        "unclosed-loop",
        RouteType::Loop,
        vec![[-98.0, 30.0], [-98.1, 30.1], [-98.2, 30.2]], // does not return to [-98.0, 30.0]
    );

    let catalog = Catalog {
        objects: vec![route],
    };
    let diags = validate_catalog(&catalog);
    assert!(diags.iter().any(|d| d.code == "loop_not_closed"));
}

#[test]
fn test_spur_detected_in_route() {
    // 400m spur in the middle
    let coords = vec![
        [-98.000, 30.000],
        [-98.000, 30.001],
        [-98.000, 30.002],
        [-97.999, 30.002],
        [-97.998, 30.002],
        [-97.999, 30.002],
        [-98.000, 30.002],
        [-98.000, 30.003],
        [-98.000, 30.004],
    ];
    let route = sample_route("spur-route", RouteType::Corridor, coords);

    let catalog = Catalog {
        objects: vec![route],
    };
    let diags = validate_catalog_with_options(&catalog, true);
    assert!(diags.iter().any(|d| d.code == "spur_detected"));
}
