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
    Validate {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        audit_spurs: bool,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Build {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,

        #[arg(long, default_value = "development")]
        environment: String,

        #[arg(long)]
        ui_dir: Option<PathBuf>,
        #[arg(long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
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
        Command::Build {
            root,
            out,

            environment,

            ui_dir,
            format,
        } => {
            let build_config = catalog_build::BuildConfig {
                root,
                out,

                environment,

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
