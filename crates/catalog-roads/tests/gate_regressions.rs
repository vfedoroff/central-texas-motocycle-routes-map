use catalog_model::{Direction, EdgeId, ObjectKey, ObjectKind};
use catalog_roads::*;
fn edge() -> RoadEdge {
    RoadEdge {
        id: EdgeId {
            way_id: 1,
            segment_index: 0,
            direction: Direction::Forward,
        },
        from: 1,
        to: 2,
        tags: [
            ("highway".into(), "primary".into()),
            ("surface".into(), "asphalt".into()),
        ]
        .into(),
        length_m: 10.0,
    }
}
#[test]
fn missing_access_cannot_pass() {
    assert!(matches!(
        classify_edge(&edge(), &[]),
        Eligibility::ReviewRequired(_)
    ));
}
#[test]
fn explicit_denial_cannot_be_reviewed_away() {
    let mut e = edge();
    e.tags.insert("access".into(), "private".into());
    let review = Review {
        edge_ids: vec![e.id.clone()],
        field: ReviewField::Access,
        asserted_value: "yes".into(),
        source_url: "https://example.org/fixture".into(),
        checked_on: "2026-10-07".into(),
        reviewer: "Synthetic fixture reviewer".into(),
        reason: "fixture".into(),
    };
    assert!(matches!(
        classify_edge(&e, &[review]),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn conditional_access_cannot_pass() {
    let mut e = edge();
    e.tags.insert("access".into(), "yes".into());
    e.tags
        .insert("motorcycle:conditional".into(), "no @ (wet)".into());
    assert!(matches!(
        classify_edge(&e, &[]),
        Eligibility::ReviewRequired(_)
    ));
}
#[test]
fn forged_network_and_schema_cannot_pass() {
    let mut e = edge();
    e.tags.insert("access".into(), "yes".into());
    let g = RoadGraph::new(vec![], vec![e.clone()], vec![]);
    let p = RoutePath {
        schema_version: 99,
        key: ObjectKey {
            kind: ObjectKind::Place,
            id: "bad".into(),
        },
        network_sha256: "forged".into(),
        junctions: vec![],
        edges: vec![e.id],
        reviews: vec![],
    };
    assert!(!validate_path(&g, &p, false).is_empty());
}
#[test]
fn absent_path_with_pass_status_cannot_verify() {
    let key = ObjectKey {
        kind: ObjectKind::Route,
        id: "fixture".into(),
    };
    let g = RoadGraph::new(vec![], vec![], vec![]);
    let c = AuditConfig::default();
    let mut e = audit_route(&key, &[], None, &g, &c, false, "2026-10-07");
    e.status = EvidenceStatus::Pass;
    assert!(
        verify_evidence(
            &e,
            &key,
            &[],
            None,
            &g.canonical_sha256(),
            &compute_config_sha256(&c),
            &current_validator_version()
        )
        .is_err()
    );
}
