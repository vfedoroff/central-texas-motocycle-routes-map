use catalog_model::{Direction, EdgeId, ObjectKey, ObjectKind};
use catalog_roads::{
    Eligibility, PathJunction, RestrictionMember, Review, ReviewField, RoadEdge, RoadGraph,
    RoadNode, RoutePath, TurnRestriction, classify_edge, validate_path,
};
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
fn make_edge(
    way_id: i64,
    segment_index: u32,
    direction: Direction,
    from: i64,
    to: i64,
    highway: &str,
    surface: Option<&str>,
    access: Option<&str>,
) -> RoadEdge {
    let mut tags = BTreeMap::new();
    tags.insert("highway".into(), highway.into());
    if let Some(s) = surface {
        tags.insert("surface".into(), s.into());
    }
    if let Some(a) = access {
        tags.insert("access".into(), a.into());
    }
    tags.insert("ref".into(), format!("Way {}", way_id));
    RoadEdge {
        id: EdgeId {
            way_id,
            segment_index,
            direction,
        },
        from,
        to,
        tags,
        length_m: 500.0,
    }
}

#[test]
fn test_clean_paved_loop_passes() {
    // Triangular loop: 1 -> 2 -> 3 -> 1
    let n1 = RoadNode {
        id: 1,
        coordinates: [-98.0, 30.0],
    };
    let n2 = RoadNode {
        id: 2,
        coordinates: [-98.0, 30.01],
    };
    let n3 = RoadNode {
        id: 3,
        coordinates: [-97.99, 30.0],
    };

    let e1 = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "primary",
        Some("asphalt"),
        Some("yes"),
    );
    let e2 = make_edge(
        20,
        0,
        Direction::Forward,
        2,
        3,
        "secondary",
        Some("paved"),
        Some("yes"),
    );
    let e3 = make_edge(
        30,
        0,
        Direction::Forward,
        3,
        1,
        "tertiary",
        Some("concrete"),
        Some("yes"),
    );

    let graph = RoadGraph::new(
        vec![n1, n2, n3],
        vec![e1.clone(), e2.clone(), e3.clone()],
        Vec::new(),
    );

    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "clean-loop".into(),
        },
        network_sha256: graph.canonical_sha256(),
        junctions: vec![
            PathJunction {
                node_id: 1,
                label: "Junction 1".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 2,
                label: "Junction 2".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 3,
                label: "Junction 3".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 1,
                label: "Junction 1 Return".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
        ],
        edges: vec![e1.id, e2.id, e3.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, true);
    assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
}

#[test]
fn test_disconnected_jump_detected() {
    // E1 ends at node 2, E2 starts at node 99 (disconnected)
    let e1 = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "primary",
        Some("asphalt"),
        None,
    );
    let e2 = make_edge(
        20,
        0,
        Direction::Forward,
        99,
        3,
        "primary",
        Some("asphalt"),
        None,
    );

    let graph = RoadGraph::new(Vec::new(), vec![e1.clone(), e2.clone()], Vec::new());
    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "jump".into(),
        },
        network_sha256: "test".into(),
        junctions: Vec::new(),
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, false);
    assert!(issues.iter().any(|i| i.code == "PATH_DISCONNECTED"));
}

#[test]
fn test_overpass_without_shared_node_rejected() {
    // Edge 1 ends at overpass node 11, edge 2 starts at underpass node 21
    let e1 = make_edge(
        1,
        0,
        Direction::Forward,
        10,
        11,
        "primary",
        Some("asphalt"),
        None,
    );
    let e2 = make_edge(
        2,
        0,
        Direction::Forward,
        21,
        22,
        "primary",
        Some("asphalt"),
        None,
    );

    let graph = RoadGraph::new(Vec::new(), vec![e1.clone(), e2.clone()], Vec::new());
    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "overpass".into(),
        },
        network_sha256: "test".into(),
        junctions: Vec::new(),
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, false);
    assert!(issues.iter().any(|i| i.code == "PATH_DISCONNECTED"));
}

#[test]
fn test_prohibited_turn_detected() {
    let e1 = make_edge(
        100,
        0,
        Direction::Forward,
        1,
        2,
        "primary",
        Some("asphalt"),
        None,
    );
    let e2 = make_edge(
        200,
        0,
        Direction::Forward,
        2,
        3,
        "secondary",
        Some("asphalt"),
        None,
    );

    let mut restr_tags = BTreeMap::new();
    restr_tags.insert("type".into(), "restriction".into());
    restr_tags.insert("restriction".into(), "no_left_turn".into());

    let members = vec![
        RestrictionMember {
            role: "from".into(),
            member_type: "way".into(),
            ref_id: 100,
        },
        RestrictionMember {
            role: "via".into(),
            member_type: "node".into(),
            ref_id: 2,
        },
        RestrictionMember {
            role: "to".into(),
            member_type: "way".into(),
            ref_id: 200,
        },
    ];

    let restriction = TurnRestriction {
        relation_id: 555,
        tags: restr_tags,
        members,
    };

    let graph = RoadGraph::new(Vec::new(), vec![e1.clone(), e2.clone()], vec![restriction]);
    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "illegal-turn".into(),
        },
        network_sha256: "test".into(),
        junctions: Vec::new(),
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, false);
    assert!(issues.iter().any(|i| i.code == "TURN_PROHIBITED"));
}

#[test]
fn test_unpaved_gravel_rejected() {
    let edge = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "tertiary",
        Some("gravel"),
        None,
    );
    let elig = classify_edge(&edge, &[]);
    assert!(matches!(elig, Eligibility::Ineligible(msg) if msg.contains("unpaved")));
}

#[test]
fn test_private_driveway_rejected() {
    let edge = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "service",
        Some("asphalt"),
        Some("private"),
    );
    let elig = classify_edge(&edge, &[]);
    assert!(matches!(elig, Eligibility::Ineligible(_)));
}

#[test]
fn test_manual_review_approves_missing_surface() {
    let edge = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "tertiary",
        None,
        Some("yes"),
    );
    // Without review: ReviewRequired
    let elig_without = classify_edge(&edge, &[]);
    assert!(matches!(elig_without, Eligibility::ReviewRequired(_)));

    // With human review evidence: Eligible
    let review = Review {
        edge_ids: vec![edge.id.clone()],
        field: ReviewField::Surface,
        asserted_value: "paved".into(),
        source_url: "https://txdot.gov".into(),
        checked_on: "2026-10-07".into(),
        reviewer: "Vadym Fedorov".into(),
        reason: "Inspected and confirmed chipseal pavement".into(),
    };
    let elig_with = classify_edge(&edge, &[review]);
    assert_eq!(elig_with, Eligibility::Eligible);
}

#[test]
fn test_unclosed_loop_rejected() {
    // 1 -> 2 -> 3 (does not return to 1)
    let e1 = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "primary",
        Some("asphalt"),
        None,
    );
    let e2 = make_edge(
        20,
        0,
        Direction::Forward,
        2,
        3,
        "primary",
        Some("asphalt"),
        None,
    );

    let graph = RoadGraph::new(Vec::new(), vec![e1.clone(), e2.clone()], Vec::new());
    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "open-loop".into(),
        },
        network_sha256: "test".into(),
        junctions: Vec::new(),
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, true); // loop=true
    assert!(issues.iter().any(|i| i.code == "LOOP_NOT_CLOSED"));
}

#[test]
fn test_junction_out_of_order_detected() {
    // Path: 1 -> 2 -> 3
    // Junctions listed in reverse order: 3 then 2
    let e1 = make_edge(
        10,
        0,
        Direction::Forward,
        1,
        2,
        "primary",
        Some("asphalt"),
        None,
    );
    let e2 = make_edge(
        20,
        0,
        Direction::Forward,
        2,
        3,
        "primary",
        Some("asphalt"),
        None,
    );

    let graph = RoadGraph::new(Vec::new(), vec![e1.clone(), e2.clone()], Vec::new());
    let path = RoutePath {
        schema_version: 1,
        key: ObjectKey {
            kind: ObjectKind::Route,
            id: "reversed-junctions".into(),
        },
        network_sha256: "test".into(),
        junctions: vec![
            PathJunction {
                node_id: 3,
                label: "Junction 3".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
            PathJunction {
                node_id: 2,
                label: "Junction 2".into(),
                group_nodes: None,
                entry_node: None,
                exit_node: None,
                review: None,
            },
        ],
        edges: vec![e1.id, e2.id],
        reviews: Vec::new(),
    };

    let issues = validate_path(&graph, &path, false);
    assert!(issues.iter().any(|i| i.code == "JUNCTION_NOT_TRAVERSED"));
}
