use catalog_model::{Direction, EdgeId};
use catalog_roads::{
    RestrictionMember, RoadEdge, RoadGraph, RoadNode, TurnRestriction, haversine_distance_m,
    is_allowed_highway, is_oneway,
};
use std::collections::BTreeMap;

#[test]
fn test_allowed_highway_filtering() {
    assert!(is_allowed_highway("motorway"));
    assert!(is_allowed_highway("trunk"));
    assert!(is_allowed_highway("primary"));
    assert!(is_allowed_highway("secondary"));
    assert!(is_allowed_highway("tertiary"));
    assert!(is_allowed_highway("unclassified"));
    assert!(is_allowed_highway("residential"));
    assert!(is_allowed_highway("primary_link"));

    // Forbidden riding classes per spec line 393
    assert!(!is_allowed_highway("service"));
    assert!(!is_allowed_highway("track"));
    assert!(!is_allowed_highway("path"));
    assert!(!is_allowed_highway("footway"));
    assert!(!is_allowed_highway("cycleway"));
    assert!(!is_allowed_highway("construction"));
}

#[test]
fn test_oneway_direction_handling() {
    let mut tags = BTreeMap::new();
    // Default two-way
    assert_eq!(is_oneway(&tags), (true, true));

    // oneway=yes
    tags.insert("oneway".into(), "yes".into());
    assert_eq!(is_oneway(&tags), (true, false));

    // oneway=-1
    tags.insert("oneway".into(), "-1".into());
    assert_eq!(is_oneway(&tags), (false, true));

    // roundabout implies oneway
    tags.clear();
    tags.insert("junction".into(), "roundabout".into());
    assert_eq!(is_oneway(&tags), (true, false));
}

#[test]
fn test_shared_node_junction_connects_ways() {
    // Way 1: Node 1 -> Node 2 -> Node 3
    // Way 2: Node 4 -> Node 2 -> Node 5
    // Shared intersection is Node 2.
    let n1 = RoadNode {
        id: 1,
        coordinates: [-98.0, 30.0],
    };
    let n2 = RoadNode {
        id: 2,
        coordinates: [-98.0, 30.01],
    }; // shared junction
    let n3 = RoadNode {
        id: 3,
        coordinates: [-98.0, 30.02],
    };
    let n4 = RoadNode {
        id: 4,
        coordinates: [-98.01, 30.01],
    };
    let n5 = RoadNode {
        id: 5,
        coordinates: [-97.99, 30.01],
    };

    let mut tags1 = BTreeMap::new();
    tags1.insert("highway".into(), "primary".into());
    tags1.insert("ref".into(), "FM 1431".into());

    let mut tags2 = BTreeMap::new();
    tags2.insert("highway".into(), "secondary".into());
    tags2.insert("ref".into(), "RM 1174".into());

    let edges = vec![
        RoadEdge {
            id: EdgeId {
                way_id: 100,
                segment_index: 0,
                direction: Direction::Forward,
            },
            from: 1,
            to: 2,
            tags: tags1.clone(),
            length_m: 1100.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 100,
                segment_index: 1,
                direction: Direction::Forward,
            },
            from: 2,
            to: 3,
            tags: tags1,
            length_m: 1100.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 200,
                segment_index: 0,
                direction: Direction::Forward,
            },
            from: 4,
            to: 2,
            tags: tags2.clone(),
            length_m: 900.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 200,
                segment_index: 1,
                direction: Direction::Forward,
            },
            from: 2,
            to: 5,
            tags: tags2,
            length_m: 900.0,
        },
    ];

    let graph = RoadGraph::new(vec![n1, n2, n3, n4, n5], edges, Vec::new());

    // Edges connected at Node 2
    let from_node_2 = graph.edges_from(2);
    assert_eq!(from_node_2.len(), 2);
    let to_node_2 = graph.edges_to(2);
    assert_eq!(to_node_2.len(), 2);

    // Node 2 allows transition from Way 100 (from 1->2) to Way 200 (to 2->5)
    assert!(from_node_2.iter().any(|e| e.id.way_id == 200 && e.to == 5));
    assert!(from_node_2.iter().any(|e| e.id.way_id == 100 && e.to == 3));
}

#[test]
fn test_overpass_without_shared_node_has_no_connection() {
    // Two ways cross geometrically at coordinate [-98.0, 30.01], but:
    // Highway 1 uses Node 10 and Node 12 (via bridge overpass at Node 11)
    // Surface road 2 uses Node 20 and Node 22 (via underpass at Node 21)
    // Node 11 and Node 21 share the same lat/lon, but have DIFFERENT node IDs!
    let n10 = RoadNode {
        id: 10,
        coordinates: [-98.0, 30.0],
    };
    let n11 = RoadNode {
        id: 11,
        coordinates: [-98.0, 30.01],
    }; // Overpass node
    let n12 = RoadNode {
        id: 12,
        coordinates: [-98.0, 30.02],
    };

    let n20 = RoadNode {
        id: 20,
        coordinates: [-98.01, 30.01],
    };
    let n21 = RoadNode {
        id: 21,
        coordinates: [-98.0, 30.01],
    }; // Underpass node (same coord, different ID!)
    let n22 = RoadNode {
        id: 22,
        coordinates: [-97.99, 30.01],
    };

    let mut tags = BTreeMap::new();
    tags.insert("highway".into(), "primary".into());

    let edges = vec![
        RoadEdge {
            id: EdgeId {
                way_id: 1,
                segment_index: 0,
                direction: Direction::Forward,
            },
            from: 10,
            to: 11,
            tags: tags.clone(),
            length_m: 1000.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 1,
                segment_index: 1,
                direction: Direction::Forward,
            },
            from: 11,
            to: 12,
            tags: tags.clone(),
            length_m: 1000.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 2,
                segment_index: 0,
                direction: Direction::Forward,
            },
            from: 20,
            to: 21,
            tags: tags.clone(),
            length_m: 1000.0,
        },
        RoadEdge {
            id: EdgeId {
                way_id: 2,
                segment_index: 1,
                direction: Direction::Forward,
            },
            from: 21,
            to: 22,
            tags,
            length_m: 1000.0,
        },
    ];

    let graph = RoadGraph::new(vec![n10, n11, n12, n20, n21, n22], edges, Vec::new());

    // From node 11 (overpass), there must be NO edge onto way 2 (underpass)!
    let from_11 = graph.edges_from(11);
    assert_eq!(from_11.len(), 1);
    assert_eq!(from_11[0].id.way_id, 1);
    assert_eq!(from_11[0].to, 12);

    // From node 21 (underpass), there must be NO edge onto way 1 (overpass)!
    let from_21 = graph.edges_from(21);
    assert_eq!(from_21.len(), 1);
    assert_eq!(from_21[0].id.way_id, 2);
    assert_eq!(from_21[0].to, 22);
}

#[test]
fn test_retained_direction_and_restrictions() {
    let mut tags = BTreeMap::new();
    tags.insert("type".into(), "restriction".into());
    tags.insert("restriction".into(), "no_left_turn".into());

    let members = vec![
        RestrictionMember {
            role: "from".into(),
            member_type: "way".into(),
            ref_id: 101,
        },
        RestrictionMember {
            role: "via".into(),
            member_type: "node".into(),
            ref_id: 55,
        },
        RestrictionMember {
            role: "to".into(),
            member_type: "way".into(),
            ref_id: 102,
        },
    ];

    let restriction = TurnRestriction {
        relation_id: 999,
        tags,
        members,
    };

    let graph = RoadGraph::new(Vec::new(), Vec::new(), vec![restriction]);
    assert_eq!(graph.restrictions.len(), 1);
    assert_eq!(graph.restrictions[0].relation_id, 999);
    assert_eq!(
        graph.restrictions[0]
            .tags
            .get("restriction")
            .map(|s| s.as_str()),
        Some("no_left_turn")
    );
    assert_eq!(graph.restrictions[0].members[1].ref_id, 55);
}

#[test]
fn test_zstd_compression_and_canonical_sha256() {
    let node = RoadNode {
        id: 42,
        coordinates: [-98.0, 30.5],
    };
    let mut tags = BTreeMap::new();
    tags.insert("highway".into(), "primary".into());
    let edge = RoadEdge {
        id: EdgeId {
            way_id: 777,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 42,
        to: 43,
        tags,
        length_m: 500.0,
    };
    let graph = RoadGraph::new(vec![node], vec![edge], Vec::new());

    // Deterministic SHA-256
    let hash1 = graph.canonical_sha256();
    let hash2 = graph.canonical_sha256();
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);

    // Zstd compression roundtrip
    let compressed = graph.encode_zstd().unwrap();
    assert!(!compressed.is_empty());
    let decoded = RoadGraph::decode_zstd(&compressed[..]).unwrap();
    assert_eq!(graph, decoded);
    assert_eq!(decoded.canonical_sha256(), hash1);
}

#[test]
fn test_haversine_distance() {
    let p1 = [-98.0, 30.0];
    let p2 = [-98.0, 30.01]; // ~1105 meters north
    let dist = haversine_distance_m(p1, p2);
    assert!((dist - 1105.0).abs() < 50.0);
}
