use anyhow::{Context, Result, bail};
use catalog_model::{CatalogPayload, ObjectKind, RouteType, load_catalog};
use catalog_roads::{
    AuditConfig, EvidenceStatus, RoadGraph, RouteAuditEvidence, RoutePath, audit_route,
};
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub struct AuditReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub review_required: usize,
    pub records: Vec<RouteAuditEvidence>,
}

pub fn run_road_audit(
    root: &Path,
    network_path: &Path,
    route_filter: Option<&str>,
    config_path: &Path,
) -> Result<AuditReport> {
    let network_file = if network_path.is_absolute() {
        network_path.to_path_buf()
    } else {
        root.join(network_path)
    };

    if !network_file.exists() {
        bail!(
            "Road network graph not found at '{}'. Run `cargo xtask network-restore` first.",
            network_file.display()
        );
    }

    let graph_bytes = fs::read(&network_file)
        .with_context(|| format!("Reading road network from '{}'", network_file.display()))?;
    let graph: RoadGraph = serde_json::from_slice(&graph_bytes).with_context(|| {
        format!(
            "Parsing road network graph from '{}'",
            network_file.display()
        )
    })?;

    let config_file = if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        root.join(config_path)
    };

    let config: AuditConfig = if config_file.exists() {
        let text = fs::read_to_string(&config_file)?;
        serde_json::from_str(&text)?
    } else {
        AuditConfig::default()
    };

    let catalog = match load_catalog(root) {
        Ok(c) => c,
        Err(diags) => {
            for d in &diags {
                eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
            }
            bail!("Failed to load catalog with {} error(s)", diags.len());
        }
    };

    let evidence_dir = root.join("content/evidence/routes");
    fs::create_dir_all(&evidence_dir)?;

    let today = "2026-10-07"; // Canonical audit run date

    let mut report = AuditReport {
        total: 0,
        passed: 0,
        failed: 0,
        review_required: 0,
        records: Vec::new(),
    };

    let mut found_route = false;

    for obj in &catalog.objects {
        if obj.key.kind != ObjectKind::Route {
            continue;
        }

        if let Some(id_filter) = route_filter {
            if obj.key.id != id_filter {
                continue;
            }
            found_route = true;
        }

        report.total += 1;
        let route_id = &obj.key.id;

        let (is_loop, track_coords) = match &obj.payload {
            CatalogPayload::Route { route_type, .. } => {
                let coords = obj.geometry.as_deref().unwrap_or(&[]);
                (*route_type == RouteType::Loop, coords)
            }
            _ => (false, [].as_slice()),
        };

        // Try to load network path if present
        let path_file = root.join(format!("content/network-paths/routes/{}.json", route_id));
        let path_record: Option<RoutePath> = if path_file.exists() {
            let path_text = fs::read_to_string(&path_file)?;
            Some(serde_json::from_str(&path_text)?)
        } else {
            None
        };

        let key = catalog_model::ObjectKey {
            kind: ObjectKind::Route,
            id: route_id.clone(),
        };

        let evidence = audit_route(
            &key,
            track_coords,
            path_record.as_ref(),
            &graph,
            &config,
            is_loop,
            today,
        );

        // Write evidence file
        let evidence_file = evidence_dir.join(format!("{}.json", route_id));
        let json_output = serde_json::to_string_pretty(&evidence)?;
        fs::write(&evidence_file, json_output)?;

        let issues_str = evidence
            .issues
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");

        match evidence.status {
            EvidenceStatus::Pass => {
                report.passed += 1;
                println!(
                    "[{:?}] {}: max deviation {:.2}m",
                    evidence.status,
                    route_id,
                    evidence.max_deviation_m.unwrap_or(0.0)
                );
            }
            EvidenceStatus::Fail => {
                report.failed += 1;
                eprintln!("[{:?}] {}: {}", evidence.status, route_id, issues_str);
            }
            EvidenceStatus::ReviewRequired => {
                report.review_required += 1;
                println!("[{:?}] {}: {}", evidence.status, route_id, issues_str);
            }
        }

        report.records.push(evidence);
    }

    if let Some(id_filter) = route_filter
        && !found_route
    {
        bail!("Requested route '{}' not found in catalog", id_filter);
    }

    Ok(report)
}
