use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    NetworkRestore {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "text")]
        format: String,
    },
    NetworkImport {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        pbf: PathBuf,
        #[arg(long, default_value = "data/road-network.lock.json")]
        lock: PathBuf,
    },
    Validate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        audit_spurs: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    AuditRoads {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "data/road-network/graph.json")]
        network: PathBuf,
        #[arg(long)]
        route: Option<String>,
        #[arg(long, default_value = "config/road-audit.json")]
        config: PathBuf,
    },
    Build {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        #[arg(long, default_value = "data/road-network/graph.json")]
        network: PathBuf,
        #[arg(long, default_value = "config/road-audit.json")]
        config: PathBuf,
        #[arg(long, default_value = "development")]
        environment: String,
        #[arg(long)]
        allow_unverified_preview: bool,
        #[arg(long)]
        ui_dir: Option<PathBuf>,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::NetworkRestore { root, format } => {
            let report = catalog_build::network_restore(&root, &format)?;
            if format == "json" {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                println!(
                    "Restored road network graph ({} nodes, {} edges, SHA-256: {})",
                    report.node_count, report.edge_count, report.graph_sha256
                );
            }
        }
        Command::NetworkImport { root, pbf, lock } => {
            // Default Central Texas bounding box
            let bounds = [-100.5, 29.0, -96.5, 31.8];
            let lockfile = catalog_build::network_import(&root, &pbf, &lock, bounds)?;
            println!(
                "Imported road network: {} nodes, SHA-256: {}",
                lockfile.parts.len(),
                lockfile.graph_sha256
            );
        }
        Command::Validate {
            root,
            audit_spurs,
            format,
        } => {
            let catalog = match catalog_model::load_catalog(&root) {
                Ok(c) => c,
                Err(diags) => {
                    if format == "json" {
                        println!("{}", serde_json::to_string(&diags)?);
                    } else {
                        for d in &diags {
                            eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
                        }
                    }
                    anyhow::bail!("Failed to load catalog with {} error(s)", diags.len());
                }
            };
            let diagnostics = catalog_model::validate_catalog_with_options(&catalog, audit_spurs);
            if !diagnostics.is_empty() {
                if format == "json" {
                    println!("{}", serde_json::to_string(&diagnostics)?);
                } else {
                    for diag in &diagnostics {
                        eprintln!(
                            "[{:?}] {}: {} ({})",
                            diag.severity, diag.code, diag.message, diag.file
                        );
                    }
                }
                anyhow::bail!(
                    "Catalog validation failed with {} diagnostic(s)",
                    diagnostics.len()
                );
            }
            if format == "json" {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "valid",
                        "route_count": catalog.objects.len()
                    })
                );
            } else {
                println!(
                    "Catalog is valid ({} routes loaded and checked).",
                    catalog.objects.len()
                );
            }
        }
        Command::AuditRoads {
            root,
            network,
            route,
            config,
        } => {
            let report = catalog_build::run_road_audit(&root, &network, route.as_deref(), &config)?;
            println!(
                "Road audit complete: {} routes ({} passed, {} failed, {} review required)",
                report.total, report.passed, report.failed, report.review_required
            );
            if report.failed > 0 || report.review_required > 0 {
                anyhow::bail!(
                    "Road audit completed with {} unverified / review required route(s)",
                    report.failed + report.review_required
                );
            }
        }
        Command::Build {
            root,
            out,
            network,
            config,
            environment,
            allow_unverified_preview,
            ui_dir,
            format,
        } => {
            let build_config = catalog_build::BuildConfig {
                root,
                out,
                network,
                config,
                environment,
                allow_unverified_preview,
                ui_dir,
            };
            catalog_build::build_site(&build_config)?;
            if format == "json" {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "success",
                        "out": build_config.out.to_string_lossy()
                    })
                );
            } else {
                println!(
                    "Site built successfully to '{}'.",
                    build_config.out.display()
                );
            }
        }
    }
    Ok(())
}
