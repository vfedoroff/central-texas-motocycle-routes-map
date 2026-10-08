use catalog_build::pages::{
    render_place_page_with_preview, render_road_page_with_preview, render_route_page_with_preview,
};
use catalog_build::{
    generate_sitemap, is_safe_link, render_markdown, render_place_page, render_route_page,
};
use catalog_model::{
    AuthorNote, CatalogObject, CatalogPayload, NearbyPlace, ObjectDetail, ObjectKey, ObjectKind,
    PlaceCategory, Recommendation, RelatedObject, RouteType, Source, StopDetail, VisitStatus,
};

#[test]
fn test_markdown_raw_html_dropped() {
    let md = "Hello <script>alert('xss')</script> world <div style='color:red'>test</div>!";
    let html = render_markdown(md);
    assert!(!html.contains("<script>"));
    assert!(!html.contains("</script>"));
    assert!(!html.contains("<div"));
    assert!(!html.contains("</div>"));
    assert!(html.contains("Hello"));
    assert!(html.contains("world"));

    // Block HTML script is completely omitted
    let block_md = "<script>\nalert('block_xss');\n</script>";
    let block_html = render_markdown(block_md);
    assert!(!block_html.contains("<script>"));
    assert!(!block_html.contains("alert('block_xss')"));
}

#[test]
fn test_markdown_images_dropped() {
    let md = "Here is an image: ![Attack](https://evil.com/tracker.png) and text.";
    let html = render_markdown(md);
    assert!(!html.contains("<img"));
    assert!(!html.contains("https://evil.com/tracker.png"));
    assert!(html.contains("Here is an image:"));
    assert!(html.contains("and text."));
}

#[test]
fn test_markdown_unsafe_links_blocked() {
    // javascript: URL
    let md_js = "[Click](javascript:alert(1))";
    let html_js = render_markdown(md_js);
    assert!(!html_js.contains("javascript:alert(1)"));

    // data: URL
    let md_data = "[Click](data:text/html,<script>alert(1)</script>)";
    let html_data = render_markdown(md_data);
    assert!(!html_data.contains("data:text/html"));

    // protocol-relative URL
    let md_proto = "[Click](//attacker.com/evil)";
    let html_proto = render_markdown(md_proto);
    assert!(!html_proto.contains("//attacker.com/evil"));

    // credentials in URL
    let md_creds = "[Click](http://user:pass@example.com/secret)";
    let html_creds = render_markdown(md_creds);
    assert!(!html_creds.contains("user:pass@"));

    // Valid safe links
    let md_safe = "[Safe](https://example.org/path) and [Relative](/routes/test/index.html)";
    let html_safe = render_markdown(md_safe);
    assert!(html_safe.contains("href=\"https://example.org/path\""));
    assert!(html_safe.contains("href=\"/routes/test/index.html\""));
}

#[test]
fn test_markdown_fenced_code_escaped() {
    let md = "```html\n<script>alert('code')</script>\n```";
    let html = render_markdown(md);
    assert!(!html.contains("<script>alert('code')</script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;/script&gt;"));
    assert!(html.contains("<pre><code"));
}

#[test]
fn test_is_safe_link_logic() {
    assert!(is_safe_link("/routes/my-route/index.html"));
    assert!(is_safe_link("https://example.com/page"));
    assert!(is_safe_link("http://example.com/page"));

    assert!(!is_safe_link("//attacker.com"));
    assert!(!is_safe_link("javascript:void(0)"));
    assert!(!is_safe_link("data:image/svg+xml,..."));
    assert!(!is_safe_link("vbscript:msgbox"));
    assert!(!is_safe_link("http://admin:secret@example.com"));
}

#[test]
fn test_route_without_optional_content() {
    let detail = ObjectDetail {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "minimal-route".into(),
        },
        title: "Minimal Route".into(),
        summary: "Just a plain route".into(),
        body_html: String::new(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        stops: vec![],
        nearby_places: vec![],
        bounds: [-98.0, 30.0, -98.0, 30.0],
        coordinates: None,
        category: Some("Twisties & Canyons".into()),
        color: Some("#e74c3c".into()),
        distance_mi: Some(15.0),
        waypoints: Some(vec!["Start".into(), "End".into()]),
        via: Some("FM 1431".into()),
        route_type: Some(RouteType::Loop),
        start_label: None,
        place_category: None,
        address: None,
        author_note: None,
        navigation_links: vec![],
        geometry_url: Some("/data/geometry/routes/minimal-route.geojson".into()),
        gpx_url: Some("/downloads/routes/minimal-route.gpx".into()),
    };

    let html = render_route_page(&detail, "https://rideatlas.org").expect("render route");
    assert!(!html.contains("unverified route"));
    assert!(
        !render_route_page_with_preview(&detail, "https://rideatlas.org", true)
            .unwrap()
            .contains("unverified route")
    );

    // Title, description, canonical
    assert!(html.contains("<title>Minimal Route - Ride Atlas</title>"));
    assert!(html.contains(
        "<link rel=\"canonical\" href=\"https://rideatlas.org/routes/minimal-route/index.html\">"
    ));
    assert!(html.contains("<meta name=\"description\" content=\"Just a plain route\">"));

    // Working map/GPX download links
    assert!(html.contains("href=\"/downloads/routes/minimal-route.gpx\""));
    assert!(html.contains("href=\"/data/geometry/routes/minimal-route.geojson\""));

    // Negative assertions: no Ride notes heading, no Route stops heading, no Near this route heading
    assert!(!html.contains("Ride notes"));
    assert!(!html.contains("Route stops"));
    assert!(!html.contains("Near this route"));
    assert!(!html.contains("<section class=\"ride-notes\">"));
    assert!(!html.contains("<section class=\"route-stops\">"));
    assert!(!html.contains("<section class=\"nearby-places\">"));
}

#[test]
fn test_route_with_optional_content_and_places() {
    let detail = ObjectDetail {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "full-route".into(),
        },
        title: "Full Route".into(),
        summary: "A route with all bells and whistles".into(),
        body_html: "<p>Rendered description</p>".into(),
        tags: vec!["scenic".into()],
        sources: vec![Source {
            title: "Texas Highways".into(),
            url: "https://texashighways.com".into(),
            accessed_on: "2026-10-07".into(),
        }],
        photos: vec![],
        related: vec![RelatedObject {
            key: ObjectKey {
                kind: ObjectKind::Place,
                id: "overlook".into(),
            },
            title: "Scenic Overlook".into(),
            page_url: "/places/overlook/index.html".into(),
        }],
        stops: vec![StopDetail {
            key: ObjectKey {
                kind: ObjectKind::Place,
                id: "historic-gas-station".into(),
            },
            title: "Historic Gas Station".into(),
            page_url: "/places/historic-gas-station/index.html".into(),
            coordinates: [-98.1, 30.2],
            note: Some("Great retro diner attached".into()),
        }],
        nearby_places: vec![NearbyPlace {
            key: ObjectKey {
                kind: ObjectKind::Place,
                id: "craft-bakery".into(),
            },
            title: "Craft Bakery".into(),
            page_url: "/places/craft-bakery/index.html".into(),
            coordinates: [-98.12, 30.22],
            distance_m: 800.0,
        }],
        bounds: [-98.2, 30.1, -98.0, 30.3],
        coordinates: None,
        category: Some("Lakes & Rivers".into()),
        color: Some("#2980b9".into()),
        distance_mi: Some(42.5),
        waypoints: Some(vec!["A".into(), "B".into()]),
        via: Some("RM 962".into()),
        route_type: Some(RouteType::Loop),
        start_label: None,
        place_category: None,
        address: None,
        author_note: Some(AuthorNote {
            visit_status: VisitStatus::Visited,
            visited_on: Some("2026-05-15".into()),
            recommendation: Some(Recommendation::Recommend),
            text: "Amazing sweepers in spring.".into(),
            updated_on: "2026-05-16".into(),
        }),
        navigation_links: vec![],
        geometry_url: Some("/data/geometry/routes/full-route.geojson".into()),
        gpx_url: Some("/downloads/routes/full-route.gpx".into()),
    };

    let html = render_route_page(&detail, "https://rideatlas.org").expect("render route");

    // Headings
    assert!(html.contains("<h2>Ride notes</h2>"));
    assert!(html.contains("<strong>Visit status:</strong> visited"));
    assert!(html.contains("<strong>Recommendation:</strong> recommend"));
    assert!(html.contains("<strong>Visited on:</strong> 2026-05-15"));
    assert!(html.contains("Amazing sweepers in spring."));

    // Stops with optional note
    assert!(html.contains("<h2>Route stops</h2>"));
    assert!(html.contains("Historic Gas Station"));
    assert!(html.contains("Great retro diner attached"));

    // Nearby places labeled as proximity
    assert!(html.contains("<h2>Near this route</h2>"));
    assert!(html.contains("Craft Bakery"));
    assert!(html.contains("(proximity)"));

    // Sources and related
    assert!(html.contains("Texas Highways"));
    assert!(html.contains("Scenic Overlook"));
}

#[test]
fn test_place_without_photos_rendered_cleanly() {
    let detail = ObjectDetail {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Place,
            id: "peaceful-park".into(),
        },
        title: "Peaceful Park".into(),
        summary: "A quiet shaded roadside park".into(),
        body_html: "<p>Shady trees and picnic tables.</p>".into(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        stops: vec![],
        nearby_places: vec![],
        bounds: [-98.5, 30.5, -98.5, 30.5],
        coordinates: Some([-98.5, 30.5]),
        category: None,
        color: None,
        distance_mi: None,
        waypoints: None,
        via: None,
        route_type: None,
        start_label: None,
        place_category: Some(PlaceCategory::Landmark),
        address: Some("123 Country Rd, TX".into()),
        author_note: None,
        navigation_links: vec![],
        geometry_url: None,
        gpx_url: None,
    };

    let html = render_place_page(&detail, "https://rideatlas.org").expect("render place");
    assert!(!html.contains("unverified route"));
    assert!(
        !render_place_page_with_preview(&detail, "https://rideatlas.org", true)
            .unwrap()
            .contains("unverified route")
    );
    let mut road = detail.clone();
    road.key.kind = ObjectKind::Road;
    assert!(
        !render_road_page_with_preview(&road, "https://rideatlas.org", false)
            .unwrap()
            .contains("unverified route")
    );
    assert!(
        !render_road_page_with_preview(&road, "https://rideatlas.org", true)
            .unwrap()
            .contains("unverified route")
    );
    assert!(html.contains("<title>Peaceful Park - Ride Atlas</title>"));
    assert!(html.contains("123 Country Rd, TX"));
    assert!(html.contains("landmark"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("Ride notes"));
}

#[test]
fn test_sitemap_generation_authored_dates_only() {
    let objects = vec![
        CatalogObject {
            schema_version: 1,
            source_path: "content/routes/route-a.json".into(),
            key: ObjectKey {
                kind: ObjectKind::Route,
                id: "route-a".into(),
            },
            title: "Route A".into(),
            summary: "Route with date".into(),
            body_markdown: String::new(),
            tags: vec![],
            sources: vec![],
            photos: vec![],
            related: vec![],
            updated_on: Some("2026-09-01".into()),
            author_note: None,
            payload: CatalogPayload::Route {
                category: "Twisties & Canyons".into(),
                color: "#e74c3c".into(),
                distance_mi: 20.0,
                waypoints: vec![],
                via: "Via".into(),
                geometry_path: "".into(),
                route_type: RouteType::Loop,
                navigation_junctions: vec![],
                stops: vec![],
            },
            geometry: None,
        },
        CatalogObject {
            schema_version: 1,
            source_path: "content/places/place-b.json".into(),
            key: ObjectKey {
                kind: ObjectKind::Place,
                id: "place-b".into(),
            },
            title: "Place B".into(),
            summary: "Place without date".into(),
            body_markdown: String::new(),
            tags: vec![],
            sources: vec![],
            photos: vec![],
            related: vec![],
            updated_on: None, // No updated_on
            author_note: None,
            payload: CatalogPayload::Place {
                place_category: PlaceCategory::Viewpoint,
                coordinates: [-98.0, 30.0],
                address: None,
                google_place_id: None,
            },
            geometry: None,
        },
    ];

    let sitemap = generate_sitemap("https://rideatlas.org", &objects);

    assert!(sitemap.contains("<loc>https://rideatlas.org/</loc>"));
    assert!(sitemap.contains("<loc>https://rideatlas.org/routes/route-a/index.html</loc>"));
    assert!(sitemap.contains("<lastmod>2026-09-01</lastmod>"));

    assert!(sitemap.contains("<loc>https://rideatlas.org/places/place-b/index.html</loc>"));
    assert!(sitemap.contains("<loc>https://rideatlas.org/privacy/index.html</loc>"));
    // Route A has lastmod, Place B does NOT have lastmod
    let place_b_entry = sitemap
        .split("<url>")
        .find(|chunk| chunk.contains("place-b"))
        .expect("place-b chunk");
    assert!(!place_b_entry.contains("<lastmod>"));
}

#[test]
fn test_privacy_page_rendering() {
    let html = catalog_build::pages::render_privacy_page(
        "https://central-texas-routes-map.netlify.app",
        false,
    )
    .expect("render privacy page");

    assert!(html.contains("<h1>Privacy Policy</h1>"));
    assert!(html.contains("catalog_object_open"));
    assert!(html.contains("catalog_filter_apply"));
    assert!(html.contains("catalog_search_empty"));
    assert!(html.contains("catalog_maps_click"));
    assert!(html.contains("catalog_gpx_download"));
    assert!(html.contains("catalog_analytics_consent"));
    assert!(html.contains("btn-allow-consent"));
    assert!(html.contains("btn-decline-consent"));
}

#[test]
fn test_route_page_navigation_and_export_disclosure() {
    let detail = ObjectDetail {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "nav-test-route".into(),
        },
        title: "Nav Test Route".into(),
        summary: "A route testing navigation and export disclosure".into(),
        body_html: String::new(),
        tags: vec![],
        sources: vec![],
        photos: vec![],
        related: vec![],
        stops: vec![],
        nearby_places: vec![],
        bounds: [-98.0, 30.0, -98.0, 30.0],
        coordinates: None,
        category: Some("Twisties & Canyons".into()),
        color: Some("#e74c3c".into()),
        distance_mi: Some(45.5),
        waypoints: None,
        via: None,
        route_type: Some(RouteType::Loop),
        start_label: Some("Oatmeal Community Center".into()),
        place_category: None,
        address: None,
        author_note: None,
        navigation_links: vec![
            catalog_model::NavigationLink {
                label: "Directions to start of Nav Test Route".into(),
                url: "https://www.google.com/maps/dir/?api=1&destination=30.0,-98.0".into(),
                mode: catalog_model::NavigationMode::Start,
            },
            catalog_model::NavigationLink {
                label: "Stage 1".into(),
                url: "https://www.google.com/maps/dir/?api=1&origin=30.0,-98.0&destination=30.5,-98.5".into(),
                mode: catalog_model::NavigationMode::Stage,
            },
        ],
        geometry_url: Some("/data/geometry/routes/nav-test-route.geojson".into()),
        gpx_url: Some("/downloads/routes/nav-test-route.gpx".into()),
    };

    let html = render_route_page(&detail, "https://rideatlas.org").expect("render route");

    // Route type and start facts
    assert!(html.contains("<strong>Route type:</strong> Loop"));
    assert!(html.contains("<strong>Starts at:</strong> Oatmeal Community Center"));

    // Navigation section with start and stage links
    assert!(html.contains("Directions to start"));
    assert!(html.contains("Starts at: Oatmeal Community Center"));
    assert!(html.contains("This gets you to the start; it does not follow the full ride."));
    assert!(html.contains("Stage 1"));

    // Export route disclosure
    assert!(html.contains("<details class=\"export-route-disclosure\">"));
    assert!(html.contains("<summary class=\"export-route-summary\">Export route</summary>"));
    assert!(html.contains("Download GPX"));
    assert!(html.contains("Download GeoJSON"));
}
