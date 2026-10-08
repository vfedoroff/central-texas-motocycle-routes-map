use crate::{
    graph::{RoadEdge, RoadGraph},
    policy::{Eligibility, Review, classify_edge},
};
use catalog_model::{EdgeId, ObjectKey, Severity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathJunction {
    pub node_id: i64,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_nodes: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_node: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_node: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<JunctionReview>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutePath {
    pub schema_version: u32,
    pub key: ObjectKey,
    pub network_sha256: String,
    pub junctions: Vec<PathJunction>,
    pub edges: Vec<EdgeId>,
    #[serde(default)]
    pub reviews: Vec<Review>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadIssue {
    pub code: String,
    pub severity: Severity,
    pub edge_id: Option<EdgeId>,
    pub node_id: Option<i64>,
    pub message: String,
}

fn issue(
    code: &str,
    severity: Severity,
    edge_id: Option<EdgeId>,
    node_id: Option<i64>,
    message: impl Into<String>,
) -> RoadIssue {
    RoadIssue {
        code: code.into(),
        severity,
        edge_id,
        node_id,
        message: message.into(),
    }
}

pub fn validate_path(graph: &RoadGraph, path: &RoutePath, is_loop: bool) -> Vec<RoadIssue> {
    let mut issues = Vec::new();
    if path.schema_version != 1
        || !matches!(
            path.key.kind,
            catalog_model::ObjectKind::Route | catalog_model::ObjectKind::Road
        )
        || path.key.id.is_empty()
    {
        issues.push(issue(
            "SCHEMA_REQUIRED",
            Severity::Error,
            None,
            None,
            "invalid path schema or key",
        ));
    }
    if path.network_sha256 != graph.canonical_sha256() {
        issues.push(issue(
            "NETWORK_HASH_MISMATCH",
            Severity::Error,
            None,
            None,
            "path does not reference this graph",
        ));
    }
    for r in &path.reviews {
        if !crate::policy::valid_review(r)
            || r.edge_ids.iter().any(|e| graph.find_edge(e).is_none())
        {
            issues.push(issue(
                "REVIEW_REQUIRED",
                Severity::Review,
                None,
                None,
                "invalid or unsourced edge review",
            ));
        }
    }
    for restriction in &graph.restrictions {
        if restriction
            .members
            .iter()
            .any(|m| m.member_type == "way" && path.edges.iter().any(|e| e.way_id == m.ref_id))
            && (restriction.tags.keys().any(|k| k.contains("conditional"))
                || restriction
                    .members
                    .iter()
                    .any(|m| m.role == "via" && m.member_type == "way")
                || restriction
                    .tags
                    .keys()
                    .any(|k| k.starts_with("restriction:"))
                || restriction.tags.contains_key("except")
                || !restriction.tags.contains_key("restriction"))
        {
            issues.push(issue(
                "REVIEW_REQUIRED",
                Severity::Review,
                None,
                None,
                "unsupported turn restriction semantics",
            ));
        }
    }

    if path.edges.is_empty() {
        issues.push(issue(
            "PATH_EMPTY",
            Severity::Error,
            None,
            None,
            "authored path contains no edges",
        ));
        return issues;
    }

    // 1. Resolve all edges and check existence
    let mut resolved_edges: Vec<&RoadEdge> = Vec::with_capacity(path.edges.len());
    for edge_id in &path.edges {
        match graph.find_edge(edge_id) {
            Some(edge) => resolved_edges.push(edge),
            None => {
                issues.push(issue(
                    "EDGE_NOT_FOUND",
                    Severity::Error,
                    Some(edge_id.clone()),
                    None,
                    format!("edge {:?} not found in reference graph", edge_id),
                ));
            }
        }
    }

    // If any edge was not found, we cannot complete topological traversal checks
    if resolved_edges.len() != path.edges.len() {
        return issues;
    }

    // 2. Continuity and duplicate checks
    for i in 0..resolved_edges.len() {
        if i + 1 < resolved_edges.len() {
            let curr = resolved_edges[i];
            let next = resolved_edges[i + 1];

            // Reject duplicate consecutive edges
            if curr.id == next.id {
                issues.push(issue(
                    "DUPLICATE_CONSECUTIVE_EDGE",
                    Severity::Error,
                    Some(curr.id.clone()),
                    None,
                    format!("duplicate consecutive edge {:?}", curr.id),
                ));
            }

            // Verify connectivity: curr.to must equal next.from
            if curr.to != next.from {
                issues.push(issue(
                    "PATH_DISCONNECTED",
                    Severity::Error,
                    Some(curr.id.clone()),
                    Some(curr.to),
                    format!(
                        "disconnected jump: edge {:?} terminates at node {}, but next edge {:?} starts at node {}",
                        curr.id, curr.to, next.id, next.from
                    ),
                ));
            }

            // 3. Turn restrictions check
            let via_node = curr.to;
            for r in &graph.restrictions {
                let Some(restr_type) = r.tags.get("restriction").map(|s| s.as_str()) else {
                    continue;
                };

                let from_match = r.members.iter().any(|m| {
                    m.role == "from" && m.member_type == "way" && m.ref_id == curr.id.way_id
                });
                let via_match = r
                    .members
                    .iter()
                    .any(|m| m.role == "via" && m.member_type == "node" && m.ref_id == via_node);
                let to_match = r.members.iter().any(|m| {
                    m.role == "to" && m.member_type == "way" && m.ref_id == next.id.way_id
                });

                if from_match && via_match {
                    match restr_type {
                        "no_left_turn" | "no_right_turn" | "no_u_turn" | "no_straight_on" => {
                            if to_match {
                                issues.push(issue(
                                    "TURN_PROHIBITED",
                                    Severity::Error,
                                    Some(curr.id.clone()),
                                    Some(via_node),
                                    format!(
                                        "prohibited turn '{}' from way {} to way {} via node {}",
                                        restr_type, curr.id.way_id, next.id.way_id, via_node
                                    ),
                                ));
                            }
                        }
                        "only_left_turn" | "only_right_turn" | "only_straight_on" => {
                            if !to_match {
                                issues.push(issue(
                                    "TURN_PROHIBITED",
                                    Severity::Error,
                                    Some(curr.id.clone()),
                                    Some(via_node),
                                    format!(
                                        "mandatory turn '{}' requires way {}, but path turned to way {}",
                                        restr_type,
                                        r.members.iter().find(|m| m.role == "to").map(|m| m.ref_id).unwrap_or(0),
                                        next.id.way_id
                                    ),
                                ));
                            }
                        }
                        _ => {
                            // Unhandled or conditional restriction
                            issues.push(issue(
                                "REVIEW_REQUIRED",
                                Severity::Review,
                                Some(curr.id.clone()),
                                Some(via_node),
                                format!(
                                    "unhandled turn restriction '{}' requires manual review",
                                    restr_type
                                ),
                            ));
                        }
                    }
                }
            }
        }

        // 4. Policy and road surface/access classification
        let edge = resolved_edges[i];
        match classify_edge(edge, &path.reviews) {
            Eligibility::Eligible => {}
            Eligibility::ReviewRequired(reason) => {
                issues.push(issue(
                    "REVIEW_REQUIRED",
                    Severity::Review,
                    Some(edge.id.clone()),
                    None,
                    reason,
                ));
            }
            Eligibility::Ineligible(reason) => {
                let code = if reason.contains("access") {
                    "ACCESS_PROHIBITED"
                } else if reason.contains("unpaved") {
                    "SURFACE_PROHIBITED"
                } else {
                    "ROAD_CLASS_PROHIBITED"
                };
                issues.push(issue(
                    code,
                    Severity::Error,
                    Some(edge.id.clone()),
                    None,
                    reason,
                ));
            }
        }
    }

    // 5. Loop closure check
    let first = resolved_edges[0];
    let last = resolved_edges[resolved_edges.len() - 1];
    if is_loop && first.from != last.to {
        issues.push(issue(
            "LOOP_NOT_CLOSED",
            Severity::Error,
            Some(last.id.clone()),
            Some(last.to),
            format!(
                "loop route must terminate at starting node {}, but terminates at node {}",
                first.from, last.to
            ),
        ));
    }

    // Controls must be actual connected road junctions, in traversal order.
    let mut nodes = vec![first.from];
    nodes.extend(resolved_edges.iter().map(|e| e.to));
    if path.junctions.is_empty() {
        issues.push(issue(
            "JUNCTION_INVALID",
            Severity::Review,
            None,
            None,
            "path requires verified endpoint junctions",
        ));
    }
    let mut cursor = 0;
    for (index, junction) in path.junctions.iter().enumerate() {
        let group = junction
            .group_nodes
            .clone()
            .unwrap_or_else(|| vec![junction.node_id]);
        let entry = junction.entry_node.unwrap_or(junction.node_id);
        let exit = junction.exit_node.unwrap_or(junction.node_id);
        let invalid = group.is_empty()
            || !group.contains(&junction.node_id)
            || !group.contains(&entry)
            || !group.contains(&exit)
            || group.iter().any(|n| graph.find_node(*n).is_none())
            || (junction.group_nodes.is_some()
                && (junction.entry_node.is_none() || junction.exit_node.is_none()))
            || !connected_group(graph, &group, &path.reviews)
            || (!automatic_junction(graph, &group, &path.reviews)
                && !reviewed_junction(graph, junction, &group))
            || junction.label.trim().is_empty();
        if invalid {
            issues.push(issue(
                "JUNCTION_INVALID",
                Severity::Review,
                None,
                Some(junction.node_id),
                "control is not an eligible junction or a valid sourced exception",
            ));
        }
        let start = nodes
            .get(cursor..)
            .and_then(|ns| ns.iter().position(|n| *n == entry))
            .map(|p| cursor + p);
        let end = start.and_then(|p| nodes[p..].iter().position(|n| *n == exit).map(|q| p + q));
        match (start, end) {
            (Some(a), Some(b)) if nodes[a..=b].iter().all(|n| group.contains(n)) => {
                if (index == 0 && a != 0)
                    || (index + 1 == path.junctions.len() && b != nodes.len() - 1)
                {
                    issues.push(issue(
                        "JUNCTION_INVALID",
                        Severity::Error,
                        None,
                        Some(junction.node_id),
                        "first/last control must match path endpoints",
                    ));
                }
                cursor = b + 1;
            }
            _ => issues.push(issue(
                "JUNCTION_NOT_TRAVERSED",
                Severity::Error,
                None,
                Some(junction.node_id),
                "junction entry/exit not traversed in order",
            )),
        }
    }

    issues
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JunctionReview {
    pub node_ids: Vec<i64>,
    pub road_ids: Vec<String>,
    pub sources: Vec<catalog_model::Source>,
    pub checked_on: String,
    pub reviewer: String,
    pub reason: String,
}
fn identity(e: &RoadEdge) -> Option<String> {
    e.tags
        .get("ref")
        .or_else(|| e.tags.get("name"))
        .map(|s| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|s| !s.is_empty())
}
fn automatic_junction(g: &RoadGraph, group: &[i64], reviews: &[Review]) -> bool {
    let mut neighbours = std::collections::BTreeSet::new();
    let mut roads = std::collections::BTreeSet::new();
    for e in &g.edges {
        if classify_edge(e, reviews) != Eligibility::Eligible {
            continue;
        }
        if group.contains(&e.from) || group.contains(&e.to) {
            if !group.contains(&e.from) {
                neighbours.insert(e.from);
            }
            if !group.contains(&e.to) {
                neighbours.insert(e.to);
            }
            if let Some(id) = identity(e) {
                roads.insert(id);
            }
        }
    }
    neighbours.len() >= 3 || roads.len() >= 2
}
fn connected_group(g: &RoadGraph, group: &[i64], reviews: &[Review]) -> bool {
    let Some(first) = group.first() else {
        return false;
    };
    let mut seen = std::collections::BTreeSet::from([*first]);
    loop {
        let n = seen.len();
        for e in &g.edges {
            if group.contains(&e.from)
                && group.contains(&e.to)
                && classify_edge(e, reviews) == Eligibility::Eligible
                && (seen.contains(&e.from) || seen.contains(&e.to))
            {
                seen.insert(e.from);
                seen.insert(e.to);
            }
        }
        if seen.len() == n {
            break;
        }
    }
    group.iter().all(|n| seen.contains(n))
}
fn reviewed_junction(g: &RoadGraph, j: &PathJunction, group: &[i64]) -> bool {
    let Some(r) = &j.review else {
        return false;
    };
    !r.reviewer.trim().is_empty()
        && !r.reason.trim().is_empty()
        && crate::policy::valid_date(&r.checked_on)
        && group.iter().all(|n| r.node_ids.contains(n))
        && r.node_ids.iter().all(|n| group.contains(n))
        && !r.road_ids.is_empty()
        && r.road_ids.iter().all(|id| {
            g.edges.iter().any(|e| {
                (group.contains(&e.from) || group.contains(&e.to))
                    && (identity(e).as_deref() == Some(id.as_str())
                        || e.id.way_id.to_string() == *id)
            })
        })
        && !r.sources.is_empty()
        && r.sources.iter().all(|s| {
            !s.title.trim().is_empty()
                && crate::policy::valid_source_url(&s.url)
                && crate::policy::valid_date(&s.accessed_on)
        })
}
