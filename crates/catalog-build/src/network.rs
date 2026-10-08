use anyhow::{Context, Result, bail};
use catalog_model::Bounds;
use catalog_roads::{RoadGraph, import_network};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt::Write as FmtWrite, fs, path::Path};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressedPart {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourcePbf {
    pub url: String,
    pub date: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadNetworkLock {
    pub schema_version: u32,
    pub graph_sha256: String,
    pub coverage_bounds: Bounds,
    pub parts: Vec<CompressedPart>,
    pub created_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pbf: Option<SourcePbf>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkRestoreReport {
    pub schema_version: u32,
    pub status: String,
    pub graph_sha256: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub restriction_count: usize,
}

fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in hash {
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

pub fn network_restore(root: &Path, _format: &str) -> Result<NetworkRestoreReport> {
    let lock_path = root.join("data/road-network.lock.json");
    if !lock_path.exists() {
        bail!("Lockfile missing at {}", lock_path.display());
    }

    let lock_content = fs::read_to_string(&lock_path)
        .with_context(|| format!("Failed to read lockfile at {}", lock_path.display()))?;
    let lock: RoadNetworkLock = serde_json::from_str(&lock_content)
        .with_context(|| format!("Failed to parse lockfile at {}", lock_path.display()))?;

    if lock.parts.is_empty() {
        bail!("Lockfile contains no compressed parts");
    }

    let mut combined_compressed = Vec::new();
    for part in &lock.parts {
        let part_file = root.join(&part.path);
        let part_bytes = fs::read(&part_file)
            .with_context(|| format!("Failed to read part file at {}", part_file.display()))?;

        if part_bytes.len() as u64 != part.size_bytes {
            bail!(
                "Part size mismatch for {}: expected {} bytes, got {}",
                part.path,
                part.size_bytes,
                part_bytes.len()
            );
        }

        let actual_hash = compute_sha256(&part_bytes);
        if actual_hash != part.sha256 {
            bail!(
                "Part checksum mismatch for {}: expected {}, got {}",
                part.path,
                part.sha256,
                actual_hash
            );
        }

        combined_compressed.extend_from_slice(&part_bytes);
    }

    // Decompress graph bytes
    let decompressed_bytes = zstd::decode_all(&combined_compressed[..])
        .context("Failed to decompress graph bytes with zstd")?;

    let graph_sha256 = compute_sha256(&decompressed_bytes);
    if graph_sha256 != lock.graph_sha256 {
        bail!(
            "Decompressed graph SHA-256 mismatch: expected {}, got {}",
            lock.graph_sha256,
            graph_sha256
        );
    }

    let graph: RoadGraph = serde_json::from_slice(&decompressed_bytes)
        .context("Failed to parse decompressed RoadGraph JSON")?;

    let out_dir = root.join("data/road-network");
    fs::create_dir_all(&out_dir)?;

    let target_graph = out_dir.join("graph.json");
    let tmp_graph = out_dir.join("graph.json.tmp");
    fs::write(&tmp_graph, &decompressed_bytes)?;
    fs::rename(&tmp_graph, &target_graph)?;

    Ok(NetworkRestoreReport {
        schema_version: 1,
        status: "pass".into(),
        graph_sha256,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        restriction_count: graph.restrictions.len(),
    })
}

pub fn network_import(
    root: &Path,
    pbf_path: &Path,
    lock_path: &Path,
    bounds: Bounds,
) -> Result<RoadNetworkLock> {
    let graph =
        import_network(pbf_path, bounds).context("Failed to import road network from PBF")?;
    let canonical_bytes = graph.canonical_bytes()?;
    let graph_sha256 = graph.canonical_sha256();

    let out_dir = root.join("data/road-network");
    fs::create_dir_all(&out_dir)?;

    let graph_json_path = out_dir.join("graph.json");
    fs::write(&graph_json_path, &canonical_bytes)?;

    let compressed_bytes = graph.encode_zstd()?;
    let part_relative = "data/road-network/graph.zst";
    let part_path = root.join(part_relative);
    fs::write(&part_path, &compressed_bytes)?;

    let part_sha256 = compute_sha256(&compressed_bytes);
    let lock = RoadNetworkLock {
        schema_version: 1,
        graph_sha256,
        coverage_bounds: bounds,
        parts: vec![CompressedPart {
            path: part_relative.into(),
            sha256: part_sha256,
            size_bytes: compressed_bytes.len() as u64,
        }],
        created_on: "2026-10-07".into(),
        source_pbf: None,
    };

    let lock_bytes = serde_json::to_vec_pretty(&lock)?;
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(lock_path, lock_bytes)?;

    Ok(lock)
}
