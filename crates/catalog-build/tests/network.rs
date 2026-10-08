use catalog_build::network::{CompressedPart, RoadNetworkLock, network_restore};
use catalog_roads::{RoadEdge, RoadGraph, RoadNode};
use std::fs;

#[test]
fn test_network_restore_validates_and_decompresses_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data/road-network");
    fs::create_dir_all(&data_dir).unwrap();

    // Create synthetic RoadGraph
    let node = RoadNode {
        id: 1001,
        coordinates: [-98.0, 30.5],
    };
    let edge = RoadEdge {
        id: catalog_model::EdgeId {
            way_id: 500,
            segment_index: 0,
            direction: catalog_model::Direction::Forward,
        },
        from: 1001,
        to: 1002,
        tags: std::collections::BTreeMap::new(),
        length_m: 250.0,
    };
    let graph = RoadGraph::new(vec![node], vec![edge], Vec::new());
    let graph_sha256 = graph.canonical_sha256();

    // Compress with zstd
    let compressed_bytes = graph.encode_zstd().unwrap();
    let part_path = data_dir.join("graph.zst");
    fs::write(&part_path, &compressed_bytes).unwrap();

    let mut hasher = sha2::Sha256::default();
    use sha2::Digest;
    hasher.update(&compressed_bytes);
    let hash = hasher.finalize();
    let part_sha256 = hash
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();

    // Write lockfile
    let lock = RoadNetworkLock {
        schema_version: 1,
        graph_sha256: graph_sha256.clone(),
        coverage_bounds: [-100.0, 29.0, -97.0, 31.0],
        parts: vec![CompressedPart {
            path: "data/road-network/graph.zst".into(),
            sha256: part_sha256,
            size_bytes: compressed_bytes.len() as u64,
        }],
        created_on: "2026-10-07".into(),
        source_pbf: None,
    };
    fs::write(
        tmp.path().join("data/road-network.lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    // Restore network
    let report = network_restore(tmp.path(), "json").unwrap();
    assert_eq!(report.status, "pass");
    assert_eq!(report.graph_sha256, graph_sha256);
    assert_eq!(report.node_count, 1);
    assert_eq!(report.edge_count, 1);

    // Verify restored file
    assert!(data_dir.join("graph.json").exists());
}

#[test]
fn test_network_restore_fails_on_checksum_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data/road-network");
    fs::create_dir_all(&data_dir).unwrap();

    let part_path = data_dir.join("graph.zst");
    fs::write(&part_path, b"corrupted bytes").unwrap();

    let lock = RoadNetworkLock {
        schema_version: 1,
        graph_sha256: "fake_hash".into(),
        coverage_bounds: [-100.0, 29.0, -97.0, 31.0],
        parts: vec![CompressedPart {
            path: "data/road-network/graph.zst".into(),
            sha256: "expected_hash_that_does_not_match".into(),
            size_bytes: 15,
        }],
        created_on: "2026-10-07".into(),
        source_pbf: None,
    };
    fs::write(
        tmp.path().join("data/road-network.lock.json"),
        serde_json::to_vec_pretty(&lock).unwrap(),
    )
    .unwrap();

    let result = network_restore(tmp.path(), "json");
    assert!(result.is_err());
    assert!(!data_dir.join("graph.json").exists());
}
