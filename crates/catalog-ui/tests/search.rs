use std::collections::HashSet;

use catalog_model::{Bounds, CatalogIndex, CatalogSummary, ObjectKey, ObjectKind, PlaceCategory};
use catalog_ui::{
    CatalogFilter, CatalogState, MileageFilterError, PAGE_SIZE, bounds_intersect,
    filter_places_overlay, filter_summaries, filter_summaries_with_saved, validate_mileage,
};

#[allow(clippy::too_many_arguments)]
fn make_summary(
    kind: ObjectKind,
    id: &str,
    title: &str,
    summary: &str,
    search_text: &str,
    bounds: Bounds,
    category: Option<&str>,
    distance_mi: Option<f64>,
) -> CatalogSummary {
    CatalogSummary {
        key: ObjectKey {
            kind,
            id: id.to_string(),
        },
        title: title.to_string(),
        summary: summary.to_string(),
        tags: vec![],
        search_text: search_text.to_string(),
        bounds,
        point: None,
        category: category.map(|s| s.to_string()),
        color: None,
        distance_mi,
        place_category: if kind == ObjectKind::Place {
            Some(PlaceCategory::Viewpoint)
        } else {
            None
        },
        page_url: format!("/objects/{id}"),
        detail_url: format!("/data/{id}.json"),
        overview_url: None,
        geometry_url: None,
        route_type: None,
        start_label: None,
    }
}

fn sample_index() -> CatalogIndex {
    CatalogIndex {
        schema_version: 1,
        objects: vec![
            make_summary(
                ObjectKind::Route,
                "twisted-sisters",
                "Twisted Sisters Circuit",
                "Scenic technical twists in Hill Country",
                "twisted sisters circuit scenic technical twists in hill country rm 335 336 337",
                [-99.9, 29.7, -99.5, 30.1],
                Some("Twisties & Canyons"),
                Some(135.5),
            ),
            make_summary(
                ObjectKind::Route,
                "willow-city-loop",
                "Willow City Loop",
                "Bluebonnets and rolling ranch road sweeps",
                "willow city loop bluebonnets and rolling ranch road sweeps fredericksburg",
                [-98.8, 30.3, -98.6, 30.5],
                Some("Plains & Ranchlands"),
                Some(45.0),
            ),
            make_summary(
                ObjectKind::Route,
                "lime-creek-road",
                "Lime Creek Road Run",
                "Short sharp bends along Lake Travis",
                "lime creek road run short sharp bends along lake travis volente",
                [-97.9, 30.4, -97.8, 30.5],
                Some("Twisties & Canyons"),
                Some(18.2),
            ),
            make_summary(
                ObjectKind::Place,
                "enchanted-rock",
                "Enchanted Rock Summit",
                "Massive pink granite dome overlook",
                "enchanted rock summit massive pink granite dome overlook fredericksburg",
                [-98.82, 30.50, -98.82, 30.50],
                None,
                None,
            ),
            make_summary(
                ObjectKind::Place,
                "oasis-viewpoint",
                "The Oasis Lake Travis",
                "Sunset overlook on the cliffside",
                "the oasis lake travis sunset overlook on the cliffside volente austin",
                [-97.87, 30.41, -97.87, 30.41],
                None,
                None,
            ),
            make_summary(
                ObjectKind::Road,
                "rm-337",
                "RM 337 West Corridor",
                "Paved switchbacks between Leakey and Vanderpool",
                "rm 337 west corridor paved switchbacks between leakey and vanderpool",
                [-99.7, 29.7, -99.5, 29.8],
                None,
                None,
            ),
        ],
    }
}

#[test]
fn test_query_and_token_matching() {
    let index = sample_index();
    let mut filter = CatalogFilter::default();

    // Empty query matches all routes (default kind = Route)
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 3);

    // Whitespace query matches all routes
    filter.query = "   \n\t  ".to_string();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 3);

    // Single token matching
    filter.query = "twisted".to_string();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "twisted-sisters");

    // Case-insensitive token
    filter.query = "TWISTED".to_string();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "twisted-sisters");

    // Multiple tokens AND matching
    filter.query = "hill country technical".to_string();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "twisted-sisters");

    // Multiple tokens where one does not match
    filter.query = "hill country nonexistent".to_string();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 0);
}

#[test]
fn test_filter_places_overlay_function() {
    let index = sample_index();
    let places = filter_places_overlay(&index, "enchanted", None);
    assert_eq!(places.len(), 1);
    assert_eq!(places[0].id, "enchanted-rock");
}

#[test]
fn test_kind_filter() {
    let index = sample_index();
    let mut filter = CatalogFilter {
        kind: ObjectKind::Place,
        ..Default::default()
    };

    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].kind, ObjectKind::Place);
    assert_eq!(res[1].kind, ObjectKind::Place);

    filter.kind = ObjectKind::Road;
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "rm-337");
}

#[test]
fn test_category_and_mileage_filters() {
    let index = sample_index();
    let mut filter = CatalogFilter {
        categories: vec!["Twisties & Canyons".to_string()],
        ..Default::default()
    };

    // Category filter on Route
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 2);
    assert!(res.iter().any(|k| k.id == "twisted-sisters"));
    assert!(res.iter().any(|k| k.id == "lime-creek-road"));

    // Min distance
    filter.min_distance_mi = Some(50.0);
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "twisted-sisters");

    // Max distance
    filter.min_distance_mi = None;
    filter.max_distance_mi = Some(30.0);
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "lime-creek-road");

    // Distance range
    filter.categories.clear();
    filter.min_distance_mi = Some(20.0);
    filter.max_distance_mi = Some(50.0);
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "willow-city-loop");

    // Category and mileage do NOT filter out places or roads even if present in filter
    filter.kind = ObjectKind::Place;
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 2);
}

#[test]
fn test_bounds_intersection_and_edge_touch() {
    // Box A: [-100.0, 30.0, -90.0, 35.0]
    let a: Bounds = [-100.0, 30.0, -90.0, 35.0];

    // Inside
    let inside: Bounds = [-95.0, 31.0, -92.0, 33.0];
    assert!(bounds_intersect(&a, &inside));

    // Overlapping
    let overlap: Bounds = [-95.0, 31.0, -85.0, 36.0];
    assert!(bounds_intersect(&a, &overlap));

    // Edge touch (east touch)
    let touch_east: Bounds = [-90.0, 30.0, -80.0, 35.0];
    assert!(bounds_intersect(&a, &touch_east));

    // Corner touch (north-east corner)
    let touch_corner: Bounds = [-90.0, 35.0, -80.0, 40.0];
    assert!(bounds_intersect(&a, &touch_corner));

    // Point touch
    let point_on_edge: Bounds = [-90.0, 32.0, -90.0, 32.0];
    assert!(bounds_intersect(&a, &point_on_edge));

    // Disjoint
    let disjoint: Bounds = [-89.0, 30.0, -80.0, 35.0];
    assert!(!bounds_intersect(&a, &disjoint));

    // Disjoint latitude
    let disjoint_lat: Bounds = [-95.0, 36.0, -92.0, 40.0];
    assert!(!bounds_intersect(&a, &disjoint_lat));
}

#[test]
fn test_bounds_filter_in_search() {
    let index = sample_index();
    let mut filter = CatalogFilter {
        applied_bounds: Some([-100.0, 29.5, -99.4, 30.2]),
        ..Default::default()
    };

    // Bounds covering only Twisted Sisters area
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "twisted-sisters");

    // Bounds covering Austin area (Lime Creek)
    filter.applied_bounds = Some([-98.0, 30.2, -97.5, 30.6]);
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "lime-creek-road");
}

#[test]
fn test_deterministic_sorting() {
    let index = CatalogIndex {
        schema_version: 1,
        objects: vec![
            make_summary(
                ObjectKind::Route,
                "route-b",
                "zebra trail",
                "",
                "zebra trail",
                [0.0, 0.0, 1.0, 1.0],
                None,
                None,
            ),
            make_summary(
                ObjectKind::Route,
                "route-a",
                "Alpha Route",
                "",
                "alpha route",
                [0.0, 0.0, 1.0, 1.0],
                None,
                None,
            ),
            make_summary(
                ObjectKind::Route,
                "route-c",
                "alpha route", // identical lowercase title, different ID
                "",
                "alpha route",
                [0.0, 0.0, 1.0, 1.0],
                None,
                None,
            ),
        ],
    };

    let filter = CatalogFilter::default();
    let res = filter_summaries(&index, &filter);
    assert_eq!(res.len(), 3);
    // alpha route (route-a) before alpha route (route-c) because id "route-a" < "route-c"
    assert_eq!(res[0].id, "route-a");
    assert_eq!(res[1].id, "route-c");
    assert_eq!(res[2].id, "route-b");
}

#[test]
fn test_saved_only_filter() {
    let index = sample_index();
    let filter = CatalogFilter {
        saved_only: true,
        ..Default::default()
    };

    let mut saved = HashSet::new();
    saved.insert(ObjectKey {
        kind: ObjectKind::Route,
        id: "willow-city-loop".to_string(),
    });

    let res = filter_summaries_with_saved(&index, &filter, Some(&saved));
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "willow-city-loop");

    // Without saved set, returns empty
    let res_none = filter_summaries(&index, &filter);
    assert_eq!(res_none.len(), 0);
}

#[test]
fn test_mileage_validation_errors() {
    // Valid cases
    assert!(validate_mileage(None, None).is_ok());
    assert!(validate_mileage(Some(0.0), Some(100.0)).is_ok());
    assert!(validate_mileage(Some(50.0), None).is_ok());
    assert!(validate_mileage(None, Some(50.0)).is_ok());

    // Negative
    assert_eq!(
        validate_mileage(Some(-1.0), None),
        Err(MileageFilterError::Negative)
    );
    assert_eq!(
        validate_mileage(None, Some(-5.0)),
        Err(MileageFilterError::Negative)
    );

    // Non-finite
    assert_eq!(
        validate_mileage(Some(f64::NAN), None),
        Err(MileageFilterError::NonFinite)
    );
    assert_eq!(
        validate_mileage(None, Some(f64::INFINITY)),
        Err(MileageFilterError::NonFinite)
    );

    // Inverted
    assert_eq!(
        validate_mileage(Some(60.0), Some(50.0)),
        Err(MileageFilterError::Inverted)
    );
}

#[test]
fn test_catalog_state_pending_vs_applied_bounds() {
    let mut state = CatalogState::new();
    let index = sample_index();

    let austin_bounds: Bounds = [-98.0, 30.2, -97.5, 30.6];
    state.set_pending_bounds(Some(austin_bounds));

    // Applied bounds still None
    assert_eq!(state.filter.applied_bounds, None);
    assert_eq!(state.pending_bounds, Some(austin_bounds));

    // Results before applying bounds return all routes
    let res = state.current_results(&index);
    assert_eq!(res.len(), 3);

    // Now apply pending bounds
    state.apply_pending_bounds();
    assert_eq!(state.filter.applied_bounds, Some(austin_bounds));

    // Results now filtered
    let res = state.current_results(&index);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "lime-creek-road");

    // Clear bounds
    state.clear_bounds();
    assert_eq!(state.filter.applied_bounds, None);
    assert_eq!(state.pending_bounds, None);
    assert_eq!(state.current_results(&index).len(), 3);
}

#[test]
fn test_catalog_state_tab_switch_remembers_route_filters_and_retains_query() {
    let mut state = CatalogState::new();

    state.set_query("scenic".to_string());
    assert!(state.set_mileage(Some(20.0), Some(100.0)).is_ok());
    state.set_categories(vec!["Twisties & Canyons".to_string()]);
    state.set_page(2);

    // Switch to Places
    state.set_kind(ObjectKind::Place);
    assert_eq!(state.filter.kind, ObjectKind::Place);
    // Query is retained
    assert_eq!(state.filter.query, "scenic");
    // Route filters cleared for Place view
    assert!(state.filter.categories.is_empty());
    assert_eq!(state.filter.min_distance_mi, None);
    assert_eq!(state.filter.max_distance_mi, None);
    // Page reset to 0
    assert_eq!(state.page, 0);

    // Switch back to Routes
    state.set_kind(ObjectKind::Route);
    assert_eq!(state.filter.kind, ObjectKind::Route);
    // Query is retained
    assert_eq!(state.filter.query, "scenic");
    // Remembered route filters are restored!
    assert_eq!(
        state.filter.categories,
        vec!["Twisties & Canyons".to_string()]
    );
    assert_eq!(state.filter.min_distance_mi, Some(20.0));
    assert_eq!(state.filter.max_distance_mi, Some(100.0));
    assert_eq!(state.page, 0);
}

#[test]
fn test_catalog_state_places_overlay_and_hidden_overlay_selection() {
    let mut state = CatalogState::new();
    let index = sample_index();

    // Default: overlay is visible, routes tab is active
    assert_eq!(state.filter.kind, ObjectKind::Route);
    assert!(state.places_overlay_visible);

    // Visible place keys returns all places matching query/bounds, ignoring route kind/categories
    let places = state.visible_place_keys(&index);
    assert_eq!(places.len(), 2);

    // When overlay is turned off: 0 places shown
    state.set_places_overlay_visible(false);
    let places = state.visible_place_keys(&index);
    assert_eq!(places.len(), 0);

    // But if a place is selected, its marker is shown even when overlay is off!
    let sel_place = ObjectKey {
        kind: ObjectKind::Place,
        id: "enchanted-rock".to_string(),
    };
    state.select_key(Some(sel_place.clone()));
    let places = state.visible_place_keys(&index);
    assert_eq!(places.len(), 1);
    assert_eq!(places[0], sel_place);

    // Clearing selection hides all places when overlay is off
    state.select_key(None);
    assert_eq!(state.visible_place_keys(&index).len(), 0);
}

#[test]
fn test_catalog_state_prior_results_context_and_back_to_route() {
    let mut state = CatalogState::new();
    state.set_query("twists".to_string());
    state.set_page(1);

    let route_key = ObjectKey {
        kind: ObjectKind::Route,
        id: "twisted-sisters".to_string(),
    };
    let place_key = ObjectKey {
        kind: ObjectKind::Place,
        id: "enchanted-rock".to_string(),
    };

    // User opens place from a route
    state.select_place_from_route(place_key.clone(), route_key.clone());
    assert_eq!(state.selected_key, Some(place_key));
    assert_eq!(state.return_route_key, Some(route_key.clone()));
    assert_eq!(state.selection_generation, 1);
    assert!(state.prior_results_context.is_some());

    // User clicks "Back to route"
    state.restore_back_to_route();
    assert_eq!(state.selected_key, Some(route_key));
    assert_eq!(state.return_route_key, None);
    assert_eq!(state.selection_generation, 2);

    // User closes route details ("Back to results")
    state.restore_back_to_results();
    assert_eq!(state.selected_key, None);
    assert_eq!(state.selection_generation, 3);
    assert_eq!(state.filter.query, "twists");
    assert_eq!(state.page, 1);
}

#[test]
fn test_pagination_and_slice() {
    let state = CatalogState::new();
    let keys: Vec<ObjectKey> = (0..75)
        .map(|i| ObjectKey {
            kind: ObjectKind::Route,
            id: format!("route-{i}"),
        })
        .collect();

    assert_eq!(state.total_pages(keys.len()), 3);
    assert_eq!(PAGE_SIZE, 30);

    let page0 = state.page_slice(&keys);
    assert_eq!(page0.len(), 30);
    assert_eq!(page0[0].id, "route-0");
    assert_eq!(page0[29].id, "route-29");

    let mut state_p1 = state.clone();
    state_p1.set_page(1);
    let page1 = state_p1.page_slice(&keys);
    assert_eq!(page1.len(), 30);
    assert_eq!(page1[0].id, "route-30");

    let mut state_p2 = state.clone();
    state_p2.set_page(2);
    let page2 = state_p2.page_slice(&keys);
    assert_eq!(page2.len(), 15);
    assert_eq!(page2[0].id, "route-60");
    assert_eq!(page2[14].id, "route-74");

    let mut state_p3 = state.clone();
    state_p3.set_page(3);
    assert_eq!(state_p3.page_slice(&keys).len(), 0);
}

#[test]
fn initial_viewport_survives_clearing_area_and_is_saved_for_selection() {
    let viewport = [-98.8, 30.0, -97.5, 31.0];
    let mut state = CatalogState {
        viewport_bounds: Some(viewport),
        ..CatalogState::default()
    };
    assert!(state.pending_bounds.is_none());
    assert!(state.filter.applied_bounds.is_none());
    state.clear_bounds();
    assert_eq!(state.viewport_bounds, Some(viewport));
    state.select_with_prior_context(ObjectKey {
        kind: ObjectKind::Route,
        id: "initial-route".into(),
    });
    assert_eq!(state.prior_viewport_bounds, Some(viewport));
}

#[test]
fn test_location_proximity_sorting() {
    let index = sample_index();
    let filter = CatalogFilter::default();

    // User located near Lake Travis / Lime Creek [-97.9, 30.45]
    let user_loc = [-97.9, 30.45];
    let keys =
        catalog_ui::search::filter_summaries_with_location(&index, &filter, None, Some(&user_loc));

    // Lime creek should be first because it's closest to [-97.9, 30.45]
    assert_eq!(keys[0].id, "lime-creek-road");

    // Test with CatalogState
    let mut state = CatalogState::default();
    state.set_user_location(Some(user_loc));
    state.set_sort_by_near_me(true);
    let state_keys = state.current_results(&index);
    assert_eq!(state_keys[0].id, "lime-creek-road");

    // When near me is toggled off, sorting returns to default title order
    state.set_sort_by_near_me(false);
    let default_keys = state.current_results(&index);
    assert_eq!(default_keys[0].id, "lime-creek-road"); // lime creek comes first alphabetically too among the 3 routes
}

#[test]
fn test_sort_result_keys_and_catalog_state_sort() {
    use catalog_ui::{CatalogSort, sort_result_keys};

    let mut index = sample_index();
    // Add route without distance
    index.objects.push(make_summary(
        ObjectKind::Route,
        "no-distance-route",
        "Alpha Route Without Distance",
        "Summary",
        "search",
        [-98.0, 30.0, -98.0, 30.0],
        Some("Twisties & Canyons"),
        None,
    ));
    // Add two routes with same distance (tie break by lowercase title)
    index.objects.push(make_summary(
        ObjectKind::Route,
        "tie-bravo",
        "Bravo Tie Route",
        "Summary",
        "search",
        [-98.0, 30.0, -98.0, 30.0],
        Some("Twisties & Canyons"),
        Some(50.0),
    ));
    index.objects.push(make_summary(
        ObjectKind::Route,
        "tie-alpha",
        "alpha tie route",
        "Summary",
        "search",
        [-98.0, 30.0, -98.0, 30.0],
        Some("Twisties & Canyons"),
        Some(50.0),
    ));

    let filter = CatalogFilter::default();
    let initial_keys = catalog_ui::filter_summaries(&index, &filter);

    // Distance ascending:
    // 18.2 (lime-creek-road), 45.0 (willow-city-loop), 50.0 ("alpha tie route" before "Bravo Tie Route"), 135.5 (twisted-sisters), None (no-distance-route)
    let asc_keys = sort_result_keys(&index, initial_keys.clone(), CatalogSort::DistanceAscending);
    let asc_ids: Vec<&str> = asc_keys.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(
        asc_ids,
        vec![
            "lime-creek-road",
            "willow-city-loop",
            "tie-alpha",
            "tie-bravo",
            "twisted-sisters",
            "no-distance-route",
        ]
    );

    // Distance descending:
    // 135.5, 50.0 ("alpha tie route" before "Bravo Tie Route"), 45.0, 18.2, None (no-distance-route)
    let desc_keys = sort_result_keys(
        &index,
        initial_keys.clone(),
        CatalogSort::DistanceDescending,
    );
    let desc_ids: Vec<&str> = desc_keys.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(
        desc_ids,
        vec![
            "twisted-sisters",
            "tie-alpha",
            "tie-bravo",
            "willow-city-loop",
            "lime-creek-road",
            "no-distance-route",
        ]
    );

    // State behavior:
    let mut state = CatalogState::default();
    state.set_page(2);
    state.set_sort_by_near_me(true);
    let gen_before = state.filter_generation;

    // set_sort disables near-me, resets page, bumps filter_generation
    state.set_sort(CatalogSort::DistanceDescending);
    assert_eq!(state.sort, CatalogSort::DistanceDescending);
    assert_eq!(state.page, 0);
    assert!(!state.sort_by_near_me);
    assert_eq!(state.filter_generation, gen_before + 1);

    // current_results uses sort
    let results = state.current_results(&index);
    assert_eq!(results[0].id, "twisted-sisters");

    // Switching to places uses Title, remembers route sort
    state.set_kind(ObjectKind::Place);
    assert_eq!(state.sort, CatalogSort::Title);
    assert_eq!(state.remembered_route_sort, CatalogSort::DistanceDescending);

    // Switching back to routes restores route sort
    state.set_kind(ObjectKind::Route);
    assert_eq!(state.sort, CatalogSort::DistanceDescending);

    // PriorResultsContext preserves sort
    state.select_with_prior_context(ObjectKey {
        kind: ObjectKind::Route,
        id: "twisted-sisters".into(),
    });
    assert_eq!(
        state.prior_results_context.as_ref().unwrap().sort,
        CatalogSort::DistanceDescending
    );
    state.sort = CatalogSort::Title; // change during selection
    state.restore_back_to_results();
    assert_eq!(state.sort, CatalogSort::DistanceDescending);

    // Clear filters resets sort to Title
    state.clear_filters();
    assert_eq!(state.sort, CatalogSort::Title);
    assert_eq!(state.remembered_route_sort, CatalogSort::Title);
}

#[test]
fn nearby_routes_use_segments_and_one_mile_threshold() {
    use catalog_ui::search::route_near_places;
    let place = [[-97.0, 31.0]];
    assert!(route_near_places(&[[-97.1, 31.0], [-96.9, 31.0]], &place));
    assert!(route_near_places(&[[-97.1, 31.01], [-96.9, 31.01]], &place));
    assert!(!route_near_places(
        &[[-97.1, 31.02], [-96.9, 31.02]],
        &place
    ));
    assert!(!route_near_places(
        &[[-97.1, 30.9], [-97.1, 31.1], [-96.9, 31.1]],
        &place
    ));
    assert!(!route_near_places(&[], &place));
}
