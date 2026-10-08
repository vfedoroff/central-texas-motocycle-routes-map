use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Ride Atlas build orchestration & maintenance tasks (replaces shell scripts)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full validation checks (fmt, clippy, tests)
    Check,
    /// Check or format code using rustfmt
    Fmt {
        /// Format files in place instead of just checking
        #[arg(long)]
        fix: bool,
    },
    /// Run clippy lints across the workspace
    Clippy,
    /// Run workspace tests
    Test,
    /// Validate catalog content and route topology
    Validate {
        /// Repository or content root
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Audit loop routes for unwanted spurs/detours
        #[arg(long, default_value_t = false)]
        audit_spurs: bool,
    },
    /// Build static site deliverables
    Build {
        /// Repository or content root
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Output directory
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Build in development mode (allows unverified previews)
        #[arg(long)]
        development: bool,
    },
    /// Restore pinned road network reference graph from compressed zstd
    NetworkRestore {
        /// Repository or content root
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Report format (text or json)
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Audit routes against reference road network and update evidence
    AuditRoads {
        /// Repository or content root
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Reference road network graph JSON
        #[arg(long, default_value = "data/road-network/graph.json")]
        network: PathBuf,
        /// Audit only a specific route ID
        #[arg(long)]
        route: Option<String>,
        /// Audit configuration JSON
        #[arg(long, default_value = "config/road-audit.json")]
        config: PathBuf,
    },
    /// Compose browser fixture environment for testing
    Fixture {
        /// Repository or content root
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

fn run_command(cmd: &mut Command, desc: &str) -> Result<()> {
    eprintln!("==> [xtask] {}", desc);
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command for {}", desc))?;
    if !status.success() {
        bail!("Command failed ({}) with exit status: {}", desc, status);
    }
    Ok(())
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn check_all() -> Result<()> {
    run_fmt(false)?;
    run_clippy()?;
    run_test()?;
    eprintln!("==> [xtask] All workspace checks passed!");
    Ok(())
}

fn run_fmt(fix: bool) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.arg("fmt").arg("--all");
    if !fix {
        cmd.arg("--").arg("--check");
    }
    run_command(
        &mut cmd,
        if fix {
            "formatting code"
        } else {
            "checking formatting"
        },
    )
}

fn run_clippy() -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]);
    run_command(&mut cmd, "checking clippy lints")
}

fn run_test() -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.args(["test", "--locked", "--workspace"]);
    run_command(&mut cmd, "running workspace tests")
}

fn run_validate(root: &Path, audit_spurs: bool) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.args([
        "run",
        "--locked",
        "-p",
        "catalog-build",
        "--",
        "validate",
        "--root",
    ]);
    cmd.arg(root);
    if audit_spurs {
        cmd.arg("--audit-spurs");
    }
    run_command(&mut cmd, "validating catalog content")
}

fn run_build(root: &Path, out: &Path, development: bool) -> Result<()> {
    // 1. Build Trunk UI into .build/ui
    let trunk_cmd = std::env::var("TRUNK").unwrap_or_else(|_| "trunk".to_string());
    let mut trunk = Command::new(trunk_cmd);
    trunk.args(["build", "--release", "--locked", "--config", "Trunk.toml"]);
    trunk.current_dir(root);
    // Trunk's clap color setting rejects NO_COLOR=1 inherited from some hosts.
    trunk.env_remove("NO_COLOR");
    run_command(&mut trunk, "building Trunk UI assets")?;

    // 2. Build static catalog site with UI composition
    let mut cmd = Command::new(cargo());
    cmd.args([
        "run",
        "--locked",
        "-p",
        "catalog-build",
        "--",
        "build",
        "--root",
    ]);
    cmd.arg(root);
    cmd.arg("--out");
    cmd.arg(out);
    cmd.arg("--ui-dir");
    cmd.arg(root.join(".build/ui"));
    cmd.arg("--format");
    cmd.arg("json");
    if development {
        cmd.args(["--environment", "development", "--allow-unverified-routes"]);
    } else {
        cmd.args(["--environment", "production", "--allow-unverified-routes"]);
    }
    run_command(&mut cmd, "building static catalog site")
}

fn run_network_restore(root: &Path, format: &str) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.args([
        "run",
        "--locked",
        "-p",
        "catalog-build",
        "--",
        "network-restore",
        "--root",
    ]);
    cmd.arg(root);
    cmd.arg("--format");
    cmd.arg(format);
    run_command(&mut cmd, "restoring reference road network")
}

fn run_audit_roads(root: &Path, network: &Path, route: Option<&str>, config: &Path) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.args([
        "run",
        "--locked",
        "-p",
        "catalog-build",
        "--",
        "audit-roads",
        "--root",
    ]);
    cmd.arg(root);
    cmd.arg("--network");
    cmd.arg(network);
    cmd.arg("--config");
    cmd.arg(config);
    if let Some(r) = route {
        cmd.arg("--route");
        cmd.arg(r);
    }
    run_command(&mut cmd, "auditing road network and generating evidence")
}

fn run_fixture(root: &Path) -> Result<()> {
    run_build(root, &root.join("dist"), true)?;
    let fixture_dir = root.join(".build/browser-fixture");
    if fixture_dir.exists() {
        std::fs::remove_dir_all(&fixture_dir)?;
    }
    copy_dir_all(&root.join("dist"), &fixture_dir)?;
    eprintln!(
        "==> [xtask] Composed browser fixture in {}",
        fixture_dir.display()
    );
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check => check_all()?,
        Commands::Fmt { fix } => run_fmt(fix)?,
        Commands::Clippy => run_clippy()?,
        Commands::Test => run_test()?,
        Commands::Validate { root, audit_spurs } => run_validate(&root, audit_spurs)?,
        Commands::Build {
            root,
            out,
            development,
        } => run_build(&root, &out, development)?,
        Commands::NetworkRestore { root, format } => run_network_restore(&root, &format)?,
        Commands::AuditRoads {
            root,
            network,
            route,
            config,
        } => run_audit_roads(&root, &network, route.as_deref(), &config)?,
        Commands::Fixture { root } => run_fixture(&root)?,
    }
    Ok(())
}
