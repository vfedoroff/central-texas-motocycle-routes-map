//! Shared read-only validation contract for lint, validate, and generation.
use crate::*;
use catalog_model::{
    Catalog, CatalogPayload, Diagnostic, ObjectKey, ObjectKind, RouteType, Severity,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const FULL_CHECKS: &[&str] = &["evidence", "network", "path", "schema", "topology"];
#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub schema_version: u32,
    pub status: String,
    pub checks: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}
impl ValidationReport {
    pub fn new(checks: Vec<String>) -> Self {
        Self {
            schema_version: 1,
            status: "pass".into(),
            checks,
            diagnostics: vec![],
        }
    }
    pub fn finish(&mut self) {
        self.checks.sort();
        self.checks.dedup();
        self.diagnostics.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.key.cmp(&b.key))
                .then(a.code.cmp(&b.code))
                .then(a.coordinate_range.cmp(&b.coordinate_range))
                .then(a.message.cmp(&b.message))
        });
        if self.status != "error"
            && self
                .diagnostics
                .iter()
                .any(|d| d.severity != Severity::Warning)
        {
            self.status = "fail".into();
        }
    }
    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "pass" => 0,
            "fail" => 1,
            _ => 2,
        }
    }
    pub fn error(&mut self, code: &str, file: &Path, message: impl Into<String>) {
        self.status = "error".into();
        self.diagnostics
            .push(diagnostic(code, Severity::Error, None, file, message));
    }
}
impl std::fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "validation {}: {}",
            self.status,
            self.diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}
impl std::error::Error for ValidationReport {}
pub fn diagnostic(
    code: &str,
    severity: Severity,
    key: Option<ObjectKey>,
    file: &Path,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity,
        key,
        file: file.to_string_lossy().into(),
        json_pointer: None,
        coordinate_range: None,
        edge_id: None,
        message: message.into(),
    }
}
pub fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        root.join(path)
    }
}
pub fn kind_dir(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Route => "routes",
        ObjectKind::Road => "roads",
        ObjectKind::Place => "places",
    }
}
pub fn safe_read(root: &Path, path: &Path) -> anyhow::Result<Vec<u8>> {
    let canonical = root.canonicalize()?;
    let actual = path.canonicalize()?;
    anyhow::ensure!(
        actual.starts_with(&canonical),
        "path escapes root: {}",
        path.display()
    );
    Ok(fs::read(actual)?)
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompressedPart {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePbf {
    pub url: String,
    pub date: String,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadNetworkLock {
    pub schema_version: u32,
    pub graph_sha256: String,
    pub coverage_bounds: catalog_model::Bounds,
    pub parts: Vec<CompressedPart>,
    pub created_on: String,
    pub source_pbf: SourcePbf,
}
pub fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_lock(lock: &RoadNetworkLock) -> anyhow::Result<()> {
    anyhow::ensure!(
        lock.schema_version == 1 && valid_hash(&lock.graph_sha256),
        "invalid graph lock schema/hash"
    );
    anyhow::ensure!(
        crate::policy::valid_date(&lock.created_on)
            && crate::policy::valid_date(
                &lock.source_pbf.date[..lock.source_pbf.date.len().min(10)]
            )
            && crate::policy::valid_source_url(&lock.source_pbf.url)
            && valid_hash(&lock.source_pbf.sha256),
        "invalid source provenance"
    );
    anyhow::ensure!(!lock.parts.is_empty(), "missing compressed graph parts");
    let mut paths = std::collections::BTreeSet::new();
    for p in &lock.parts {
        anyhow::ensure!(
            p.size_bytes > 0
                && p.size_bytes <= 20_000_000
                && valid_hash(&p.sha256)
                && paths.insert(&p.path)
                && !Path::new(&p.path).is_absolute()
                && Path::new(&p.path)
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "invalid compressed graph part"
        );
    }
    anyhow::ensure!(
        lock.coverage_bounds.iter().all(|v| v.is_finite())
            && lock.coverage_bounds[0] < lock.coverage_bounds[2]
            && lock.coverage_bounds[1] < lock.coverage_bounds[3],
        "invalid coverage bounds"
    );
    Ok(())
}
pub fn load_verified_graph(root: &Path, path: &Path) -> anyhow::Result<RoadGraph> {
    let lock: RoadNetworkLock =
        serde_json::from_slice(&safe_read(root, &root.join("data/road-network.lock.json"))?)?;
    validate_lock(&lock)?;
    let bytes = safe_read(root, &resolve(root, path))?;
    anyhow::ensure!(
        crate::evidence::sha256_hex(&bytes) == lock.graph_sha256,
        "NETWORK_HASH_MISMATCH: graph bytes differ from lock"
    );
    let graph: RoadGraph = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        graph.schema_version == 1 && graph.canonical_sha256() == lock.graph_sha256,
        "NETWORK_HASH_MISMATCH: graph is not canonical"
    );
    anyhow::ensure!(
        graph.nodes.windows(2).all(|n| n[0].id < n[1].id)
            && graph.edges.windows(2).all(|e| e[0].id < e[1].id),
        "graph identities must be unique and sorted"
    );
    anyhow::ensure!(
        graph.edges.iter().all(|e| graph.find_node(e.from).is_some()
            && graph.find_node(e.to).is_some()
            && e.from != e.to
            && e.length_m.is_finite()
            && e.length_m > 0.0),
        "invalid graph edge endpoints"
    );
    Ok(graph)
}
pub fn load_config(root: &Path, path: &Path) -> anyhow::Result<AuditConfig> {
    let path = resolve(root, path);
    if path.exists() {
        Ok(serde_json::from_slice(&safe_read(root, &path)?)?)
    } else {
        Ok(AuditConfig::default())
    }
}
pub fn validate(
    root: &Path,
    network: &Path,
    config_path: &Path,
    routes: &[String],
    requested: Option<&str>,
) -> ValidationReport {
    let checks: Vec<String> = requested
        .map(|s| s.split(',').map(str::to_owned).collect())
        .unwrap_or_else(|| FULL_CHECKS.iter().map(|s| s.to_string()).collect());
    let mut report = ValidationReport::new(checks);
    if report.checks.is_empty()
        || report
            .checks
            .iter()
            .any(|s| !FULL_CHECKS.contains(&s.as_str()))
    {
        report.error(
            "SCHEMA_REQUIRED",
            root,
            "unknown or empty --checks; use schema,topology,network,path,evidence",
        );
        return report;
    }
    let catalog = match catalog_model::load_catalog(root) {
        Ok(c) => c,
        Err(d) => {
            report.status = "error".into();
            report.diagnostics = d;
            report.finish();
            return report;
        }
    };
    for id in routes {
        if !catalog
            .objects
            .iter()
            .any(|o| o.key.kind == ObjectKind::Route && o.key.id == *id)
        {
            report.error(
                "REFERENCE_MISSING",
                root,
                format!("unknown selected route {id}"),
            );
        }
    }
    if report.exit_code() == 2 {
        report.finish();
        return report;
    }
    if report
        .checks
        .iter()
        .any(|s| s == "schema" || s == "topology")
    {
        report.diagnostics.extend(
            catalog_model::validate_catalog_with_options(
                &catalog,
                report.checks.iter().any(|s| s == "topology"),
            )
            .into_iter()
            .filter(|d| {
                routes.is_empty()
                    || d.key
                        .as_ref()
                        .is_none_or(|k| k.kind == ObjectKind::Route && routes.contains(&k.id))
            }),
        );
    }
    if report
        .checks
        .iter()
        .any(|s| s == "network" || s == "path" || s == "evidence")
    {
        let graph = match load_verified_graph(root, network) {
            Ok(g) => g,
            Err(e) => {
                let code = if e.to_string().contains("HASH_MISMATCH") {
                    "NETWORK_HASH_MISMATCH"
                } else {
                    "NETWORK_MISSING"
                };
                report.error(code, &resolve(root, network), e.to_string());
                report.finish();
                return report;
            }
        };
        let config = match load_config(root, config_path) {
            Ok(c) => c,
            Err(e) => {
                report.error("SCHEMA_REQUIRED", config_path, e.to_string());
                report.finish();
                return report;
            }
        };
        validate_lines(root, &catalog, &graph, &config, routes, &mut report);
    }
    report.finish();
    report
}
fn validate_lines(
    root: &Path,
    catalog: &Catalog,
    graph: &RoadGraph,
    config: &AuditConfig,
    routes: &[String],
    report: &mut ValidationReport,
) {
    let check_path = report.checks.iter().any(|s| s == "path" || s == "evidence");
    if !check_path {
        return;
    }
    for obj in &catalog.objects {
        if obj.key.kind == ObjectKind::Place
            || (!routes.is_empty()
                && (obj.key.kind != ObjectKind::Route || !routes.contains(&obj.key.id)))
        {
            continue;
        }
        let key = &obj.key;
        let dir = kind_dir(key.kind);
        let file = root.join(format!("content/network-paths/{dir}/{}.json", key.id));
        let path: RoutePath =
            match safe_read(root, &file).and_then(|b| Ok(serde_json::from_slice(&b)?)) {
                Ok(p) => p,
                Err(e) => {
                    if file.exists() {
                        report.error("SCHEMA_REQUIRED", &file, e.to_string());
                    } else {
                        report.diagnostics.push(diagnostic(
                            "REVIEW_REQUIRED",
                            Severity::Review,
                            Some(key.clone()),
                            &file,
                            "missing authored network path",
                        ));
                    }
                    continue;
                }
            };
        let is_loop = matches!(
            &obj.payload,
            CatalogPayload::Route {
                route_type: RouteType::Loop,
                ..
            }
        );
        let coords = obj.geometry.as_deref().unwrap_or(&[]);
        // Independently rerun topology, policy and alignment; a saved pass cannot bypass them.
        let current = audit_route(
            key,
            coords,
            Some(&path),
            graph,
            config,
            is_loop,
            "2000-01-01",
        );
        for mut d in current.issues {
            d.file = file.to_string_lossy().into();
            report.diagnostics.push(d);
        }
        if let CatalogPayload::Route {
            navigation_junctions,
            ..
        } = &obj.payload
        {
            let mut cursor = 0;
            for nav in navigation_junctions {
                if let Some(p) = path.junctions[cursor..]
                    .iter()
                    .position(|j| j.node_id == nav.node_id)
                {
                    cursor += p + 1;
                } else {
                    report.diagnostics.push(diagnostic(
                        "JUNCTION_INVALID",
                        Severity::Error,
                        Some(key.clone()),
                        &file,
                        "navigation junction not in ordered path controls",
                    ));
                }
            }
        }
        if report.checks.iter().any(|s| s == "evidence") {
            let file = root.join(format!("content/audits/{dir}/{}.json", key.id));
            match safe_read(root, &file)
                .and_then(|b| Ok(serde_json::from_slice::<RouteAuditEvidence>(&b)?))
            {
                Ok(e) => {
                    if let Err(e) = verify_evidence(
                        &e,
                        key,
                        coords,
                        Some(&path),
                        &graph.canonical_sha256(),
                        &compute_config_sha256(config),
                        &current_validator_version(),
                    ) {
                        report.diagnostics.push(diagnostic(
                            "AUDIT_STALE",
                            Severity::Review,
                            Some(key.clone()),
                            &file,
                            e.to_string(),
                        ));
                    }
                }
                Err(e) => {
                    if file.exists() {
                        report.error("SCHEMA_REQUIRED", &file, e.to_string());
                    } else {
                        report.diagnostics.push(diagnostic(
                            "AUDIT_STALE",
                            Severity::Review,
                            Some(key.clone()),
                            &file,
                            "missing audit evidence",
                        ));
                    }
                }
            }
        }
    }
}
pub fn print_report(report: &ValidationReport, format: &str) {
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string(report).expect("report serialization")
        );
    } else if format == "github" {
        for d in &report.diagnostics {
            let level = if d.severity == Severity::Warning {
                "warning"
            } else {
                "error"
            };
            println!(
                "::{level} file={},title={}::{}",
                github_escape(&d.file, true),
                github_escape(&d.code, true),
                github_escape(&d.message, false)
            );
        }
    } else {
        for d in &report.diagnostics {
            eprintln!("{}: {} ({})", d.code, d.message, d.file);
        }
        println!("Validation {}", report.status);
    }
}
pub fn github_escape(s: &str, property: bool) -> String {
    let s = s
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A");
    if property {
        s.replace(':', "%3A").replace(',', "%2C")
    } else {
        s
    }
}
