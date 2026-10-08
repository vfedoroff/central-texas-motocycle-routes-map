use anyhow::Result;
use catalog_model::{
    CatalogPayload, Diagnostic, ObjectKind, RouteType, Severity, load_catalog,
    validate_catalog_with_options,
};
use catalog_roads::{
    AuditConfig, RoadGraph, RouteAuditEvidence, RoutePath, compute_config_sha256,
    current_validator_version, validate_path, verify_evidence,
};
use clap::Parser;
use serde::Serialize;
use std::{fs, path::PathBuf, process};

#[derive(Parser, Debug)]
#[command(
    name = "route-lint",
    about = "Ride Atlas read-only road and catalog linter"
)]
struct Cli {
    /// Root directory of repository
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Pinned road network reference graph
    #[arg(long, default_value = "data/road-network/graph.json")]
    network: PathBuf,

    /// Selected route ID to check (can be specified multiple times)
    #[arg(long = "route")]
    routes: Vec<String>,

    /// Road audit configuration JSON
    #[arg(long, default_value = "config/road-audit.json")]
    config: PathBuf,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    format: String,

    /// Filter checks to run (comma separated)
    #[arg(long)]
    checks: Option<String>,
}

#[derive(Serialize, Debug)]
struct LintReport {
    schema_version: u32,
    status: String,
    checks: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

fn diagnostic(
    code: &str,
    severity: Severity,
    key: Option<catalog_model::ObjectKey>,
    file: &str,
    coordinate_range: Option<[usize; 2]>,
    edge_id: Option<catalog_model::EdgeId>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        key,
        file: file.to_string(),
        json_pointer: None,
        coordinate_range,
        edge_id,
        message: message.into(),
    }
}

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(exit_code) => process::exit(exit_code),
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(2);
        }
    }
}

fn run(cli: Cli) -> Result<i32> {
    let root = &cli.root;
    let is_json = cli.format == "json";

    let check_filter: Option<Vec<String>> = cli
        .checks
        .as_ref()
        .map(|c| c.split(',').map(|s| s.trim().to_string()).collect());
    let schema_only = check_filter.as_deref() == Some(&["schema".to_string()]);

    // 1. Load catalog
    let catalog = match load_catalog(root) {
        Ok(c) => c,
        Err(diags) => {
            if is_json {
                let report = LintReport {
                    schema_version: 1,
                    status: "error".into(),
                    checks: vec!["catalog_load".into()],
                    diagnostics: diags,
                };
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for d in &diags {
                    eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
                }
            }
            return Ok(2);
        }
    };

    // Check requested route IDs exist
    if !cli.routes.is_empty() {
        for req_id in &cli.routes {
            let found = catalog
                .objects
                .iter()
                .any(|obj| obj.key.kind == ObjectKind::Route && obj.key.id == *req_id);
            if !found {
                eprintln!("Error: Unknown requested route ID '{}'", req_id);
                return Ok(2);
            }
        }
    }

    let mut diagnostics = Vec::new();

    // 2. Schema and content validation (including spur topology check)
    let schema_diags = validate_catalog_with_options(&catalog, true);
    for d in schema_diags {
        if cli.routes.is_empty()
            || d.key
                .as_ref()
                .map(|k| cli.routes.contains(&k.id))
                .unwrap_or(false)
        {
            diagnostics.push(d);
        }
    }

    let mut checks = if schema_only {
        vec!["schema".to_string()]
    } else {
        vec![
            "access_policy".to_string(),
            "audit_evidence".to_string(),
            "geometry_validity".to_string(),
            "loop_closure".to_string(),
            "path_connectivity".to_string(),
            "road_alignment".to_string(),
            "schema_validation".to_string(),
            "surface_policy".to_string(),
            "topology_spurs".to_string(),
            "turn_restrictions".to_string(),
        ]
    };
    checks.sort();

    if !schema_only {
        // 3. Verify network file
        let network_path = if cli.network.is_absolute() {
            cli.network.clone()
        } else {
            root.join(&cli.network)
        };

        if !network_path.exists() {
            if is_json {
                let report = LintReport {
                    schema_version: 1,
                    status: "error".into(),
                    checks: vec!["network_presence".into()],
                    diagnostics: vec![diagnostic(
                        "NETWORK_MISSING",
                        Severity::Error,
                        None,
                        network_path.to_str().unwrap_or(""),
                        None,
                        None,
                        format!("Road network file not found: '{}'", network_path.display()),
                    )],
                };
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!(
                    "Error: Road network file not found at '{}'. Run `cargo xtask network-restore` first.",
                    network_path.display()
                );
            }
            return Ok(2);
        }

        let graph_bytes = fs::read(&network_path)?;
        let graph: RoadGraph = match serde_json::from_slice(&graph_bytes) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Failed to parse road graph JSON: {}", e);
                return Ok(2);
            }
        };

        // 4. Load audit config
        let config_path = if cli.config.is_absolute() {
            cli.config.clone()
        } else {
            root.join(&cli.config)
        };

        let audit_config: AuditConfig = if config_path.exists() {
            let text = fs::read_to_string(&config_path)?;
            serde_json::from_str(&text)?
        } else {
            AuditConfig::default()
        };

        let graph_sha256 = graph.canonical_sha256();
        let config_sha256 = compute_config_sha256(&audit_config);
        let val_ver = current_validator_version();

        for obj in &catalog.objects {
            if obj.key.kind != ObjectKind::Route {
                continue;
            }

            if !cli.routes.is_empty() && !cli.routes.contains(&obj.key.id) {
                continue;
            }

            let route_id = &obj.key.id;
            let route_key = obj.key.clone();
            let (is_loop, track_coords) = match &obj.payload {
                CatalogPayload::Route { route_type, .. } => {
                    let coords = obj.geometry.as_deref().unwrap_or(&[]);
                    (*route_type == RouteType::Loop, coords)
                }
                _ => (false, [].as_slice()),
            };

            // Check path file
            let path_file = root.join(format!("content/network-paths/routes/{}.json", route_id));
            let path_record: Option<RoutePath> = if path_file.exists() {
                match fs::read_to_string(&path_file) {
                    Ok(text) => match serde_json::from_str(&text) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            diagnostics.push(diagnostic(
                                "PATH_INVALID",
                                Severity::Error,
                                Some(route_key.clone()),
                                path_file.to_str().unwrap_or(""),
                                None,
                                None,
                                format!("Failed to parse network path JSON: {}", e),
                            ));
                            None
                        }
                    },
                    Err(e) => {
                        diagnostics.push(diagnostic(
                            "PATH_UNREADABLE",
                            Severity::Error,
                            Some(route_key.clone()),
                            path_file.to_str().unwrap_or(""),
                            None,
                            None,
                            format!("Failed to read path file: {}", e),
                        ));
                        None
                    }
                }
            } else {
                None
            };

            // Check evidence file
            let evidence_file = root.join(format!("content/evidence/routes/{}.json", route_id));
            if !evidence_file.exists() {
                diagnostics.push(diagnostic(
                    "AUDIT_STALE",
                    Severity::Review,
                    Some(route_key.clone()),
                    &obj.source_path,
                    None,
                    None,
                    format!(
                        "Route '{}' has not been audited against road network",
                        route_id
                    ),
                ));
            } else {
                match fs::read_to_string(&evidence_file) {
                    Ok(text) => match serde_json::from_str::<RouteAuditEvidence>(&text) {
                        Ok(evidence) => {
                            if let Err(err) = verify_evidence(
                                &evidence,
                                &route_key,
                                track_coords,
                                path_record.as_ref(),
                                &graph_sha256,
                                &config_sha256,
                                &val_ver,
                            ) {
                                diagnostics.push(diagnostic(
                                    "AUDIT_STALE",
                                    Severity::Error,
                                    Some(route_key.clone()),
                                    evidence_file.to_str().unwrap_or(""),
                                    None,
                                    None,
                                    format!("Audit evidence invalid: {}", err),
                                ));
                            }
                        }
                        Err(e) => {
                            diagnostics.push(diagnostic(
                                "AUDIT_INVALID",
                                Severity::Error,
                                Some(route_key.clone()),
                                evidence_file.to_str().unwrap_or(""),
                                None,
                                None,
                                format!("Malformed audit evidence: {}", e),
                            ));
                        }
                    },
                    Err(e) => {
                        diagnostics.push(diagnostic(
                            "AUDIT_UNREADABLE",
                            Severity::Error,
                            Some(route_key.clone()),
                            evidence_file.to_str().unwrap_or(""),
                            None,
                            None,
                            format!("Failed to read audit evidence: {}", e),
                        ));
                    }
                }
            }

            // Validate path independently
            if let Some(path) = &path_record {
                let issues = validate_path(&graph, path, is_loop);
                for issue in issues {
                    let code = match issue.code.as_str() {
                        "direction_prohibited" => "DIRECTION_PROHIBITED",
                        "turn_prohibited" => "TURN_PROHIBITED",
                        "surface_prohibited" => "SURFACE_PROHIBITED",
                        "access_prohibited" => "ACCESS_PROHIBITED",
                        "disconnected_path" => "PATH_DISCONNECTED",
                        "junction_out_of_order" | "invalid_junction_node" => "JUNCTION_INVALID",
                        _ => "REVIEW_REQUIRED",
                    };
                    diagnostics.push(diagnostic(
                        code,
                        issue.severity,
                        Some(route_key.clone()),
                        path_file.to_str().unwrap_or(""),
                        None,
                        issue.edge_id,
                        issue.message,
                    ));
                }
            }
        }
    }

    // Sort diagnostics deterministically: file, kind/ID, code, coordinate_range, message
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.key.cmp(&b.key))
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.coordinate_range.cmp(&b.coordinate_range))
            .then_with(|| a.message.cmp(&b.message))
    });

    let has_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
    let has_reviews = diagnostics.iter().any(|d| d.severity == Severity::Review);

    let status = if has_errors {
        "fail"
    } else if has_reviews {
        "review_required"
    } else {
        "pass"
    };

    if is_json {
        let report = LintReport {
            schema_version: 1,
            status: status.to_string(),
            checks,
            diagnostics,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for d in &diagnostics {
            eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
        }
        if status == "pass" {
            println!("All route checks passed cleanly.");
        } else {
            println!(
                "Route lint status: {} ({} diagnostic(s))",
                status,
                diagnostics.len()
            );
        }
    }

    if status == "pass" { Ok(0) } else { Ok(1) }
}
