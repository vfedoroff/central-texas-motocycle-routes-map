use crate::{
    AlignmentStatus, AuditConfig, Coordinate, RoadGraph, RoutePath, compare_track, validate_path,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const EVIDENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Pass,
    Fail,
    ReviewRequired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteAuditEvidence {
    pub schema_version: u32,
    pub key: catalog_model::ObjectKey,
    pub status: EvidenceStatus,
    pub checked_on: String,
    pub validator_version: String,
    pub geometry_sha256: String,
    pub route_path_sha256: Option<String>,
    pub graph_sha256: String,
    pub config_sha256: String,
    pub max_deviation_m: Option<f64>,
    pub issues: Vec<catalog_model::Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    MissingEvidence,
    ForgedRouteId {
        expected: String,
        found: String,
    },
    StaleGeometry {
        expected: String,
        found: String,
    },
    StalePath {
        expected: Option<String>,
        found: Option<String>,
    },
    StaleGraph {
        expected: String,
        found: String,
    },
    StaleConfig {
        expected: String,
        found: String,
    },
    StaleValidator {
        expected: String,
        found: String,
    },
    AuditFailed(String),
    ReviewRequired(String),
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceError::MissingEvidence => write!(f, "Missing road audit evidence"),
            EvidenceError::ForgedRouteId { expected, found } => {
                write!(
                    f,
                    "Forged route ID in evidence: expected {}, found {}",
                    expected, found
                )
            }
            EvidenceError::StaleGeometry { expected, found } => {
                write!(
                    f,
                    "Stale geometry hash in evidence: expected {}, found {}",
                    expected, found
                )
            }
            EvidenceError::StalePath { expected, found } => {
                write!(
                    f,
                    "Stale path hash in evidence: expected {:?}, found {:?}",
                    expected, found
                )
            }
            EvidenceError::StaleGraph { expected, found } => {
                write!(
                    f,
                    "Stale road graph hash in evidence: expected {}, found {}",
                    expected, found
                )
            }
            EvidenceError::StaleConfig { expected, found } => {
                write!(
                    f,
                    "Stale audit config hash in evidence: expected {}, found {}",
                    expected, found
                )
            }
            EvidenceError::StaleValidator { expected, found } => {
                write!(
                    f,
                    "Stale validator version in evidence: expected {}, found {}",
                    expected, found
                )
            }
            EvidenceError::AuditFailed(msg) => write!(f, "Audit recorded failure: {}", msg),
            EvidenceError::ReviewRequired(msg) => {
                write!(f, "Audit recorded review required: {}", msg)
            }
        }
    }
}

impl std::error::Error for EvidenceError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in hash {
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

pub fn compute_geometry_sha256(coords: &[Coordinate]) -> String {
    let json = serde_json::to_string(&serde_json::json!({"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":coords}})).expect("finite geometry");
    sha256_hex(json.as_bytes())
}

pub fn compute_path_sha256(path: &RoutePath) -> String {
    let json =
        serde_json::to_string(&serde_json::to_value(path).expect("path JSON")).expect("path JSON");
    sha256_hex(json.as_bytes())
}

pub fn compute_config_sha256(config: &AuditConfig) -> String {
    let json = serde_json::to_string(&serde_json::to_value(config).expect("config JSON"))
        .expect("config JSON");
    sha256_hex(json.as_bytes())
}

pub fn current_validator_version() -> String {
    let pkg_version = env!("CARGO_PKG_VERSION");
    let sources_hash = sha256_hex(
        concat!(
            include_str!("policy.rs"),
            include_str!("path.rs"),
            include_str!("geometry.rs"),
            include_str!("graph.rs"),
            include_str!("evidence.rs"),
            include_str!("gate.rs"),
            include_str!("../../catalog-model/src/validation.rs")
        )
        .as_bytes(),
    );
    format!("{}-{}", pkg_version, sources_hash)
}

/// Perform complete road audit on a route and generate verified evidence.
pub fn audit_route(
    key: &catalog_model::ObjectKey,
    track_coords: &[Coordinate],
    path: Option<&RoutePath>,
    graph: &RoadGraph,
    config: &AuditConfig,
    is_loop: bool,
    checked_on: &str,
) -> RouteAuditEvidence {
    let geom_hash = compute_geometry_sha256(track_coords);
    let path_hash = path.map(compute_path_sha256);
    let graph_hash = graph.canonical_sha256();
    let config_hash = compute_config_sha256(config);
    let val_version = current_validator_version();

    let mut issues = Vec::new();

    let route_path = match path {
        Some(p) => p,
        None => {
            issues.push(audit_issue(
                key,
                "REVIEW_REQUIRED",
                catalog_model::Severity::Review,
                "No road path sequence authored",
            ));
            return RouteAuditEvidence {
                schema_version: EVIDENCE_SCHEMA_VERSION,
                key: key.clone(),
                status: EvidenceStatus::ReviewRequired,
                checked_on: checked_on.to_string(),
                validator_version: val_version,
                geometry_sha256: geom_hash,
                route_path_sha256: None,
                graph_sha256: graph_hash,
                config_sha256: config_hash,
                max_deviation_m: None,
                issues,
            };
        }
    };

    // 1. Validate topological path
    let road_issues = validate_path(graph, route_path, is_loop);
    let mut has_review = false;
    let mut has_fail = !crate::policy::valid_date(checked_on) || route_path.key != *key;
    if has_fail {
        issues.push(audit_issue(
            key,
            "SCHEMA_REQUIRED",
            catalog_model::Severity::Error,
            "invalid audit date or mismatched path key",
        ));
    }

    for issue in road_issues {
        if issue.severity == catalog_model::Severity::Review {
            has_review = true;
        } else {
            has_fail = true;
        }
        issues.push(catalog_model::Diagnostic {
            code: issue.code,
            severity: issue.severity,
            key: Some(key.clone()),
            file: String::new(),
            json_pointer: None,
            coordinate_range: None,
            edge_id: issue.edge_id,
            message: issue.message,
        });
    }

    // 2. Build path polyline from graph nodes and edges
    let node_map: std::collections::HashMap<_, _> =
        graph.nodes.iter().map(|n| (n.id, n.coordinates)).collect();
    let edge_map: std::collections::HashMap<_, _> =
        graph.edges.iter().map(|e| (e.id.clone(), e)).collect();

    let mut path_line: Vec<Coordinate> = Vec::new();
    for edge_id in &route_path.edges {
        if let Some(edge) = edge_map.get(edge_id) {
            if let Some(&from_coord) = node_map.get(&edge.from)
                && (path_line.is_empty() || path_line.last() != Some(&from_coord))
            {
                path_line.push(from_coord);
            }
            if let Some(&to_coord) = node_map.get(&edge.to) {
                path_line.push(to_coord);
            }
        }
    }

    // 3. Compare whole track to path line
    let align_report = compare_track(track_coords, &path_line, config);
    if align_report.status == AlignmentStatus::Fail {
        has_fail = true;
        issues.push(audit_issue(
            key,
            if align_report.status == AlignmentStatus::Fail {
                "TRACK_OFF_NETWORK"
            } else {
                "REVIEW_REQUIRED"
            },
            if align_report.status == AlignmentStatus::Fail {
                catalog_model::Severity::Error
            } else {
                catalog_model::Severity::Review
            },
            &align_report.message,
        ));
    } else if align_report.status == AlignmentStatus::ReviewRequired {
        has_review = true;
        issues.push(audit_issue(
            key,
            if align_report.status == AlignmentStatus::Fail {
                "TRACK_OFF_NETWORK"
            } else {
                "REVIEW_REQUIRED"
            },
            if align_report.status == AlignmentStatus::Fail {
                catalog_model::Severity::Error
            } else {
                catalog_model::Severity::Review
            },
            &align_report.message,
        ));
    }

    let status = if has_fail {
        EvidenceStatus::Fail
    } else if has_review {
        EvidenceStatus::ReviewRequired
    } else {
        EvidenceStatus::Pass
    };

    RouteAuditEvidence {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        key: key.clone(),
        status,
        checked_on: checked_on.to_string(),
        validator_version: val_version,
        geometry_sha256: geom_hash,
        route_path_sha256: path_hash,
        graph_sha256: graph_hash,
        config_sha256: config_hash,
        max_deviation_m: align_report.max_deviation_m,
        issues,
    }
}

/// Verify existing evidence against current state and hashes for build gate check.
pub fn verify_evidence(
    evidence: &RouteAuditEvidence,
    expected_key: &catalog_model::ObjectKey,
    track_coords: &[Coordinate],
    path: Option<&RoutePath>,
    expected_graph_sha256: &str,
    expected_config_sha256: &str,
    expected_validator_version: &str,
) -> Result<(), EvidenceError> {
    if evidence.key != *expected_key {
        return Err(EvidenceError::ForgedRouteId {
            expected: format!("{:?}", expected_key),
            found: format!("{:?}", evidence.key),
        });
    }
    if evidence.schema_version != 1 || !crate::policy::valid_date(&evidence.checked_on) {
        return Err(EvidenceError::AuditFailed(
            "invalid schema or checked_on date".into(),
        ));
    }
    if evidence.graph_sha256 != expected_graph_sha256 {
        return Err(EvidenceError::StaleGraph {
            expected: expected_graph_sha256.to_string(),
            found: evidence.graph_sha256.clone(),
        });
    }

    if evidence.config_sha256 != expected_config_sha256 {
        return Err(EvidenceError::StaleConfig {
            expected: expected_config_sha256.to_string(),
            found: evidence.config_sha256.clone(),
        });
    }

    if evidence.validator_version != expected_validator_version {
        return Err(EvidenceError::StaleValidator {
            expected: expected_validator_version.to_string(),
            found: evidence.validator_version.clone(),
        });
    }

    if path.is_none_or(|p| {
        p.edges.is_empty()
            || p.key != *expected_key
            || p.schema_version != 1
            || p.network_sha256 != expected_graph_sha256
    }) {
        return Err(EvidenceError::AuditFailed(
            "missing, empty or mismatched path".into(),
        ));
    }
    if evidence.status == EvidenceStatus::Pass
        && (!evidence.issues.is_empty()
            || evidence
                .max_deviation_m
                .is_none_or(|v| !v.is_finite() || v < 0.0))
    {
        return Err(EvidenceError::AuditFailed(
            "inconsistent pass evidence".into(),
        ));
    }

    let current_geom_hash = compute_geometry_sha256(track_coords);
    if evidence.geometry_sha256 != current_geom_hash {
        return Err(EvidenceError::StaleGeometry {
            expected: current_geom_hash,
            found: evidence.geometry_sha256.clone(),
        });
    }

    let current_path_hash = path.map(compute_path_sha256);
    if evidence.route_path_sha256 != current_path_hash {
        return Err(EvidenceError::StalePath {
            expected: current_path_hash,
            found: evidence.route_path_sha256.clone(),
        });
    }

    match evidence.status {
        EvidenceStatus::Pass => Ok(()),
        EvidenceStatus::Fail => Err(EvidenceError::AuditFailed(
            evidence
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )),
        EvidenceStatus::ReviewRequired => Err(EvidenceError::ReviewRequired(
            evidence
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )),
    }
}

pub fn audit_issue(
    key: &catalog_model::ObjectKey,
    code: &str,
    severity: catalog_model::Severity,
    message: &str,
) -> catalog_model::Diagnostic {
    catalog_model::Diagnostic {
        code: code.into(),
        severity,
        key: Some(key.clone()),
        file: String::new(),
        json_pointer: None,
        coordinate_range: None,
        edge_id: None,
        message: message.into(),
    }
}
