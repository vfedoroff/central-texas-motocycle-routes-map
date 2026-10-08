use catalog_model::{Direction, EdgeId, ObjectKey, ObjectKind};
use catalog_roads::{
    AuditConfig, Coordinate, EvidenceError, EvidenceStatus, PathJunction, RoadEdge, RoadGraph,
    RoadNode, RoutePath, audit_route, compute_config_sha256, current_validator_version,
    verify_evidence,
};
use std::collections::BTreeMap;

fn make_test_graph() -> (RoadGraph, Vec<Coordinate>, RoutePath) {
    let p1: Coordinate = [-98.00, 30.00];
    let p2: Coordinate = [-98.00, 30.01];
    let p3: Coordinate = [-97.99, 30.01];

    let n1 = RoadNode {
        id: 1,
        coordinates: p1,
    };
    let n2 = RoadNode {
        id: 2,
        coordinates: p2,
    };
    let n3 = RoadNode {
        id: 3,
        coordinates: p3,
    };
    let n10 = RoadNode {
        id: 10,
        coordinates: [-98.01, 30.00],
    };
    let n30 = RoadNode {
        id: 30,
        coordinates: [-97.98, 30.01],
    };

    let mut tags1 = BTreeMap::new();
    tags1.insert("highway".into(), "primary".into());
    tags1.insert("surface".into(), "asphalt".into());
    tags1.insert("access".into(), "yes".into());
    tags1.insert("ref".into(), "Road A".into());

    let mut tags2 = BTreeMap::new();
    tags2.insert("highway".into(), "primary".into());
    tags2.insert("surface".into(), "asphalt".into());
    tags2.insert("access".into(), "yes".into());
    tags2.insert("ref".into(), "Road B".into());

    let mut tags_cross1 = BTreeMap::new();
    tags_cross1.insert("highway".into(), "primary".into());
    tags_cross1.insert("surface".into(), "asphalt".into());
    tags_cross1.insert("access".into(), "yes".into());
    tags_cross1.insert("ref".into(), "Cross A".into());

    let mut tags_cross3 = BTreeMap::new();
    tags_cross3.insert("highway".into(), "primary".into());
    tags_cross3.insert("surface".into(), "asphalt".into());
    tags_cross3.insert("access".into(), "yes".into());
    tags_cross3.insert("ref".into(), "Cross B".into());

    let e1 = RoadEdge {
        id: EdgeId {
            way_id: 10,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 1,
        to: 2,
        tags: tags1,
        length_m: 1100.0,
    };

    let e2 = RoadEdge {
        id: EdgeId {
            way_id: 20,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 2,
        to: 3,
        tags: tags2,
        length_m: 1000.0,
    };

    let e_cross1 = RoadEdge {
        id: EdgeId {
            way_id: 100,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 10,
        to: 1,
        tags: tags_cross1,
        length_m: 500.0,
    };

    let e_cross3 = RoadEdge {
        id: EdgeId {
            way_id: 300,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 3,
        to: 30,
        tags: tags_cross3,
        length_m: 500.0,
    };

    let graph = RoadGraph::new(
        vec![n1, n2, n3, n10, n30],
        vec![e1.clone(), e2.clone(), e_cross1, e_cross3],
        Vec::new(),
    );
    let track = vec![p1, p2, p3];

    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "test-route".into(),
        },
        network_sha256: graph.canonical_sha256(),
        junctions: vec![
            PathJunction {
                node_id: 1,
                label: "Start".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 2,
                label: "Turn".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 3,
                label: "End".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
        ],
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    (graph, track, path)
}

#[test]
fn test_passing_unchanged_record() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let checked_on = "2026-10-07";

    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        checked_on,
    );

    eprintln!("ISSUES: {:?}", evidence.issues);
    assert_eq!(evidence.status, EvidenceStatus::Pass);
    assert!(evidence.max_deviation_m.is_some());
    assert!(evidence.issues.is_empty());

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&path),
        &graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(res.is_ok());
}

#[test]
fn test_changed_geometry_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let mut modified_track = track.clone();
    modified_track[1] = [-98.05, 30.01]; // modified coordinate

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &modified_track,
        Some(&path),
        &graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::StaleGeometry { .. })));
}

#[test]
fn test_changed_ordered_path_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let mut modified_path = path.clone();
    modified_path.edges.reverse(); // altered order

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&modified_path),
        &graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::StalePath { .. })));
}

#[test]
fn test_changed_snapshot_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let different_graph_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let mut path_with_new_graph = path.clone();
    path_with_new_graph.network_sha256 = different_graph_hash.to_string();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&path_with_new_graph),
        different_graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::StaleGraph { .. })));
}

#[test]
fn test_changed_config_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let different_config_hash = "1111111111111111111111111111111111111111111111111111111111111111";
    let graph_hash = graph.canonical_sha256();
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&path),
        &graph_hash,
        different_config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::StaleConfig { .. })));
}

#[test]
fn test_forged_route_id_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let forged_key = ObjectKey {
        kind: ObjectKind::Route,
        id: "forged-route".into(),
    };
    let genuine_key = ObjectKey {
        kind: ObjectKind::Route,
        id: "genuine-route".into(),
    };
    let evidence = audit_route(
        &forged_key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &genuine_key,
        &track,
        Some(&path),
        &graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::ForgedRouteId { .. })));
}

#[test]
fn test_stale_validator_version_rejected() {
    let (graph, track, path) = make_test_graph();
    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let different_val_ver = "0.0.1-stale";

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&path),
        &graph_hash,
        &config_hash,
        different_val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::StaleValidator { .. })));
}

#[test]
fn test_review_required_surface_fails_gate() {
    let (mut graph, track, mut path) = make_test_graph();
    // Clear surface tag so edge requires review
    graph.edges[0].tags.remove("surface");
    path.network_sha256 = graph.canonical_sha256();

    let config = AuditConfig::default();
    let evidence = audit_route(
        &path.key,
        &track,
        Some(&path),
        &graph,
        &config,
        false,
        "2026-10-07",
    );

    assert_eq!(evidence.status, EvidenceStatus::ReviewRequired);

    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(&config);
    let val_ver = current_validator_version();

    let res = verify_evidence(
        &evidence,
        &path.key,
        &track,
        Some(&path),
        &graph_hash,
        &config_hash,
        &val_ver,
    );
    assert!(matches!(res, Err(EvidenceError::ReviewRequired(_))));
}
