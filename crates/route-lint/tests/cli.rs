use std::{fs, process::Command};
use tempfile::tempdir;

fn run_route_lint(args: &[&str]) -> (i32, String, String) {
    let cargo = env!("CARGO");
    let mut cmd_args = vec!["run", "--locked", "-p", "route-lint", "--"];
    cmd_args.extend_from_slice(args);

    let output = Command::new(cargo)
        .args(&cmd_args)
        .output()
        .expect("Failed to execute route-lint");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

fn create_test_catalog(root: &std::path::Path) {
    fs::create_dir_all(root.join("content/routes")).unwrap();
    fs::create_dir_all(root.join("content/geometry/routes")).unwrap();

    let route_json = r##"{
        "schema_version": 1,
        "id": "sample-route",
        "title": "Sample Route",
        "summary": "A sample route",
        "category": "Twisties & Canyons",
        "color": "#e74c3c",
        "distance_mi": 10.0,
        "waypoints": ["Start", "End"],
        "via": "Road A",
        "geometry_path": "content/geometry/routes/sample-route.geojson",
        "route_type": "loop",
        "tags": ["scenic"]
    }"##;
    fs::write(root.join("content/routes/sample-route.json"), route_json).unwrap();

    let p1 = [-98.00, 30.00];
    let p2 = [-98.00, 30.01];
    let geojson = serde_json::json!({
        "type": "Feature",
        "properties": {},
        "geometry": {
            "type": "LineString",
            "coordinates": [p1, p2, p1]
        }
    });
    fs::write(
        root.join("content/geometry/routes/sample-route.geojson"),
        serde_json::to_string(&geojson).unwrap(),
    )
    .unwrap();
}

#[test]
fn test_missing_graph_exits_2() {
    let dir = tempdir().unwrap();
    create_test_catalog(dir.path());

    let (code, stdout, stderr) = run_route_lint(&[
        "--root",
        dir.path().to_str().unwrap(),
        "--network",
        "nonexistent/graph.json",
        "--format",
        "json",
    ]);

    assert_eq!(code, 2);
    assert!(stdout.contains("NETWORK_MISSING") || stderr.contains("not found"));
}

#[test]
fn test_unknown_route_id_exits_2() {
    let dir = tempdir().unwrap();
    create_test_catalog(dir.path());

    let graph_path = dir.path().join("graph.json");
    fs::write(
        &graph_path,
        r#"{"schema_version":1,"nodes":[],"edges":[],"restrictions":[]}"#,
    )
    .unwrap();

    let (code, stdout, stderr) = run_route_lint(&[
        "--root",
        dir.path().to_str().unwrap(),
        "--network",
        graph_path.to_str().unwrap(),
        "--route",
        "nonexistent-route-id-xyz",
    ]);

    assert_eq!(code, 2);
    assert!(
        stderr.contains("Unknown requested route ID")
            || stdout.contains("Unknown requested route ID")
    );
}

#[test]
fn test_clean_route_fixture_passes_exit_0() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create minimal valid catalog
    fs::create_dir_all(root.join("content/routes")).unwrap();
    fs::create_dir_all(root.join("content/geometry/routes")).unwrap();
    fs::create_dir_all(root.join("content/evidence/routes")).unwrap();
    fs::create_dir_all(root.join("content/network-paths/routes")).unwrap();

    let route_json = r##"{
        "schema_version": 1,
        "id": "clean-loop",
        "title": "Clean Loop",
        "summary": "A clean loop",
        "category": "Twisties & Canyons",
        "color": "#e74c3c",
        "distance_mi": 10.0,
        "waypoints": ["Start", "End"],
        "via": "Road A",
        "geometry_path": "content/geometry/routes/clean-loop.geojson",
        "route_type": "loop",
        "tags": ["scenic"]
    }"##;
    fs::write(root.join("content/routes/clean-loop.json"), route_json).unwrap();

    let p1 = [-98.00, 30.00];
    let p2 = [-98.00, 30.01];
    let p3 = [-97.99, 30.01];
    let geojson = serde_json::json!({
        "type": "Feature",
        "properties": {},
        "geometry": {
            "type": "LineString",
            "coordinates": [p1, p2, p3, p1]
        }
    });
    fs::write(
        root.join("content/geometry/routes/clean-loop.geojson"),
        serde_json::to_string(&geojson).unwrap(),
    )
    .unwrap();

    // Graph
    let graph = catalog_roads::RoadGraph::new(
        vec![
            catalog_roads::RoadNode {
                id: 1,
                coordinates: p1,
            },
            catalog_roads::RoadNode {
                id: 2,
                coordinates: p2,
            },
            catalog_roads::RoadNode {
                id: 3,
                coordinates: p3,
            },
        ],
        vec![
            catalog_roads::RoadEdge {
                id: catalog_model::EdgeId {
                    way_id: 1,
                    segment_index: 0,
                    direction: catalog_model::Direction::Forward,
                },
                from: 1,
                to: 2,
                tags: [
                    ("highway".into(), "primary".into()),
                    ("surface".into(), "asphalt".into()),
                    ("access".into(), "yes".into()),
                    ("ref".into(), "Road 1".into()),
                ]
                .into(),
                length_m: 1100.0,
            },
            catalog_roads::RoadEdge {
                id: catalog_model::EdgeId {
                    way_id: 2,
                    segment_index: 0,
                    direction: catalog_model::Direction::Forward,
                },
                from: 2,
                to: 3,
                tags: [
                    ("highway".into(), "primary".into()),
                    ("surface".into(), "asphalt".into()),
                    ("access".into(), "yes".into()),
                    ("ref".into(), "Road 2".into()),
                ]
                .into(),
                length_m: 1000.0,
            },
            catalog_roads::RoadEdge {
                id: catalog_model::EdgeId {
                    way_id: 3,
                    segment_index: 0,
                    direction: catalog_model::Direction::Forward,
                },
                from: 3,
                to: 1,
                tags: [
                    ("highway".into(), "primary".into()),
                    ("surface".into(), "asphalt".into()),
                    ("access".into(), "yes".into()),
                    ("ref".into(), "Road 3".into()),
                ]
                .into(),
                length_m: 1200.0,
            },
        ],
        Vec::new(),
    );
    let graph_path = root.join("graph.json");
    fs::write(&graph_path, serde_json::to_string(&graph).unwrap()).unwrap();

    // Network path
    let path = catalog_roads::RoutePath {
        schema_version: 1,
        key: catalog_model::ObjectKey {
            kind: catalog_model::ObjectKind::Route,
            id: "clean-loop".into(),
        },
        network_sha256: graph.canonical_sha256(),
        junctions: vec![
            catalog_roads::PathJunction {
                node_id: 1,
                label: "Start".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            catalog_roads::PathJunction {
                node_id: 2,
                label: "Turn 1".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            catalog_roads::PathJunction {
                node_id: 3,
                label: "Turn 2".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            catalog_roads::PathJunction {
                node_id: 1,
                label: "Return".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
        ],
        edges: vec![
            catalog_model::EdgeId {
                way_id: 1,
                segment_index: 0,
                direction: catalog_model::Direction::Forward,
            },
            catalog_model::EdgeId {
                way_id: 2,
                segment_index: 0,
                direction: catalog_model::Direction::Forward,
            },
            catalog_model::EdgeId {
                way_id: 3,
                segment_index: 0,
                direction: catalog_model::Direction::Forward,
            },
        ],
        reviews: Vec::new(),
    };
    fs::write(
        root.join("content/network-paths/routes/clean-loop.json"),
        serde_json::to_string(&path).unwrap(),
    )
    .unwrap();

    // Audit evidence
    let config = catalog_roads::AuditConfig::default();
    let route_key = catalog_model::ObjectKey {
        kind: catalog_model::ObjectKind::Route,
        id: "clean-loop".into(),
    };
    let evidence = catalog_roads::audit_route(
        &route_key,
        &[p1, p2, p3, p1],
        Some(&path),
        &graph,
        &config,
        true,
        "2026-10-07",
    );
    assert_eq!(evidence.status, catalog_roads::EvidenceStatus::Pass);
    fs::write(
        root.join("content/evidence/routes/clean-loop.json"),
        serde_json::to_string_pretty(&evidence).unwrap(),
    )
    .unwrap();

    // Run route-lint
    let (code, stdout, _) = run_route_lint(&[
        "--root",
        root.to_str().unwrap(),
        "--network",
        graph_path.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert_eq!(code, 0);
    assert!(stdout.contains(r#""status": "pass""#));
}

#[test]
fn test_unverified_missing_evidence_exits_1() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("content/routes")).unwrap();
    fs::create_dir_all(root.join("content/geometry/routes")).unwrap();

    let route_json = r##"{
        "schema_version": 1,
        "id": "unverified-route",
        "title": "Unverified Route",
        "summary": "A route without evidence",
        "category": "Twisties & Canyons",
        "color": "#e74c3c",
        "distance_mi": 5.0,
        "waypoints": ["A", "B"],
        "via": "Road",
        "geometry_path": "content/geometry/routes/unverified-route.geojson",
        "route_type": "corridor",
        "tags": []
    }"##;
    fs::write(
        root.join("content/routes/unverified-route.json"),
        route_json,
    )
    .unwrap();

    let p1 = [-98.00, 30.00];
    let p2 = [-98.00, 30.01];
    let geojson = serde_json::json!({
        "type": "Feature",
        "properties": {},
        "geometry": {
            "type": "LineString",
            "coordinates": [p1, p2]
        }
    });
    fs::write(
        root.join("content/geometry/routes/unverified-route.geojson"),
        serde_json::to_string(&geojson).unwrap(),
    )
    .unwrap();

    let graph = catalog_roads::RoadGraph::new(Vec::new(), Vec::new(), Vec::new());
    let graph_path = root.join("graph.json");
    fs::write(&graph_path, serde_json::to_string(&graph).unwrap()).unwrap();

    let (code, stdout, _) = run_route_lint(&[
        "--root",
        root.to_str().unwrap(),
        "--network",
        graph_path.to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert_eq!(code, 1);
    assert!(stdout.contains("AUDIT_STALE"));
}

#[test]
fn schema_subset_does_not_require_graph() {
    let dir = tempdir().unwrap();
    create_test_catalog(dir.path());
    let (code, out, _) = run_route_lint(&[
        "--root",
        dir.path().to_str().unwrap(),
        "--checks",
        "schema",
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "{out}");
    let report: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(report["checks"], serde_json::json!(["schema"]));
}
