mod common;
use anyhow::Result;
use catalog_build::{BuildConfig, build_index, build_site};
use catalog_model::{
    Catalog, CatalogIndex, CatalogObject, CatalogPayload, ObjectDetail, ObjectKey, ObjectKind,
    RouteType,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use tempfile::tempdir;

fn setup_fixture_catalog(root: &Path) -> Result<()> {
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2024"
"#,
    )?;

    fs::create_dir_all(root.join("content/routes"))?;
    fs::create_dir_all(root.join("content/geometry/routes"))?;
    fs::create_dir_all(root.join("content/places"))?;

    // Route geometry
    let geojson = r#"{
  "type": "Feature",
  "properties": {},
  "geometry": {
    "type": "LineString",
    "coordinates": [
      [-98.0, 30.0],
      [-98.1, 30.1],
      [-98.2, 30.1],
      [-98.0, 30.0]
    ]
  }
}"#;
    fs::write(
        root.join("content/geometry/routes/sample-circuit.geojson"),
        geojson,
    )?;

    // Route record
    let route = r##"{
  "schema_version": 1,
  "id": "sample-circuit",
  "title": "Sample Circuit",
  "summary": "A scenic test circuit.",
  "category": "Twisties & Canyons",
  "color": "#e74c3c",
  "distance_mi": 35.0,
  "waypoints": ["Start", "Point B", "Start"],
  "via": "Scenic FM 1431",
  "geometry_path": "content/geometry/routes/sample-circuit.geojson",
  "route_type": "loop",
  "tags": ["scenic", "canyon"],
  "sources": []
}"##;
    fs::write(root.join("content/routes/sample-circuit.json"), route)?;

    // Place record
    let place = r#"{
  "schema_version": 1,
  "id": "lookout-point",
  "title": "Lookout Point",
  "summary": "Scenic view overlooking the canyon.",
  "place_category": "viewpoint",
  "coordinates": [-98.1, 30.1],
  "tags": ["view"]
}"#;
    fs::write(root.join("content/places/lookout-point.json"), place)?;

    common::seed_map_cache(root)?;
    Ok(())
}

fn compute_directory_file_hashes(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut file_hashes = BTreeMap::new();
    let mut queue = vec![dir.to_path_buf()];

    while let Some(current) = queue.pop() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                queue.push(path);
            } else if path.is_file() {
                let rel = path.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
                let bytes = fs::read(&path)?;
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let hash_bytes = hasher.finalize();
                let mut hash = String::with_capacity(64);
                for b in hash_bytes {
                    use std::fmt::Write;
                    let _ = write!(hash, "{:02x}", b);
                }
                file_hashes.insert(rel, hash);
            }
        }
    }

    Ok(file_hashes)
}

#[test]
fn test_full_generator_referential_integrity_and_determinism() -> Result<()> {
    let dir1 = tempdir()?;
    let root1 = dir1.path().to_path_buf();
    setup_fixture_catalog(&root1)?;
    let out1 = root1.join("dist");

    let cfg1 = BuildConfig {
        root: root1.clone(),
        out: out1.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg1)?;

    // 1. Verify index and referenced local URLs exist on disk
    let index_bytes = fs::read(out1.join("catalog-index.json"))?;
    let index: CatalogIndex = serde_json::from_slice(&index_bytes)?;
    assert!(!index.objects.is_empty());

    for summary in &index.objects {
        // page_url must exist
        let page_rel = summary.page_url.trim_start_matches('/');
        assert!(
            out1.join(page_rel).exists(),
            "Missing page on disk: {}",
            summary.page_url
        );

        // detail_url must exist
        let detail_rel = summary.detail_url.trim_start_matches('/');
        assert!(
            out1.join(detail_rel).exists(),
            "Missing detail JSON on disk: {}",
            summary.detail_url
        );

        // overview_url must exist if present
        if let Some(overview) = &summary.overview_url {
            let ov_rel = overview.trim_start_matches('/');
            assert!(
                out1.join(ov_rel).exists(),
                "Missing overview on disk: {}",
                overview
            );
        }

        // geometry_url must exist if present
        if let Some(geom) = &summary.geometry_url {
            let geom_rel = geom.trim_start_matches('/');
            assert!(
                out1.join(geom_rel).exists(),
                "Missing geometry on disk: {}",
                geom
            );
        }
    }

    // 2. Verify detail references (GPX download, sitemap, HTML pages)
    let route_detail_bytes = fs::read(out1.join("data/routes/sample-circuit.json"))?;
    let route_detail: ObjectDetail = serde_json::from_slice(&route_detail_bytes)?;
    if let Some(gpx) = &route_detail.gpx_url {
        let gpx_rel = gpx.trim_start_matches('/');
        assert!(out1.join(gpx_rel).exists(), "Missing GPX download: {}", gpx);
    }

    assert!(out1.join("sitemap.xml").exists());
    assert!(out1.join("privacy/index.html").exists());
    assert!(out1.join("data/site-config.json").exists());

    // 3. Verify no unsafe source files leaked into dist
    assert!(!out1.join(".git").exists());
    assert!(!out1.join("Cargo.toml").exists());
    assert!(!out1.join("content").exists());
    assert!(!out1.join("crates").exists());

    // 4. Build a second time in an independent directory and verify byte-for-byte reproducibility
    let dir2 = tempdir()?;
    let root2 = dir2.path().to_path_buf();
    setup_fixture_catalog(&root2)?;
    let out2 = root2.join("dist");

    let cfg2 = BuildConfig {
        root: root2.clone(),
        out: out2.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg2)?;

    let hashes1 = compute_directory_file_hashes(&out1)?;
    let hashes2 = compute_directory_file_hashes(&out2)?;

    assert_eq!(
        hashes1.keys().collect::<Vec<_>>(),
        hashes2.keys().collect::<Vec<_>>()
    );

    for (file, hash1) in &hashes1 {
        let hash2 = &hashes2[file];
        assert_eq!(
            hash1, hash2,
            "File '{}' differs between runs: {} vs {}",
            file, hash1, hash2
        );
    }

    Ok(())
}

#[test]
fn test_synthetic_1000_object_fixture_index_size() {
    let mut objects = Vec::with_capacity(1000);

    for i in 0..1000 {
        let is_route = i % 2 == 0;
        let id = format!("obj-{:04}", i);
        let title = format!("Scenic Route or Landmark {:04} with a Bounded Name", i);
        let summary = format!(
            "Summary {:04} describing this motorcycle destination in Central Texas with hill country views.",
            i
        );
        let tags = vec![
            "scenic".to_string(),
            "hill-country".to_string(),
            "paved".to_string(),
        ];

        let payload = if is_route {
            CatalogPayload::Route {
                category: "Twisties & Canyons".to_string(),
                color: "#e74c3c".to_string(),
                distance_mi: 45.5,
                waypoints: vec!["Start".to_string(), "Way B".to_string(), "End".to_string()],
                via: "TX 29 West -> RM 1174 North -> RM 963 East".to_string(),
                geometry_path: format!("content/geometry/routes/{}.geojson", id),
                route_type: RouteType::Loop,
                navigation_junctions: vec![],
                stops: vec![],
            }
        } else {
            CatalogPayload::Place {
                place_category: catalog_model::PlaceCategory::Viewpoint,
                coordinates: [-98.0 + (i as f64 * 0.001), 30.0 + (i as f64 * 0.001)],
                address: Some(format!("{} County Rd 100, Burnet, TX", 100 + i)),
                google_place_id: None,
            }
        };

        objects.push(CatalogObject {
            schema_version: 1,
            source_path: format!("content/synthetic/{}.json", id),
            key: ObjectKey {
                kind: if is_route {
                    ObjectKind::Route
                } else {
                    ObjectKind::Place
                },
                id,
            },
            title,
            summary,
            body_markdown: String::new(),
            tags,
            sources: vec![],
            photos: vec![],
            related: vec![],
            updated_on: Some("2026-10-07".to_string()),
            author_note: None,
            payload,
            geometry: if is_route {
                Some(vec![[-98.0, 30.0], [-98.1, 30.1], [-98.0, 30.0]])
            } else {
                None
            },
        });
    }

    let catalog = Catalog { objects };
    let index = build_index(&catalog);
    let index_json = serde_json::to_string(&index).expect("serialize index");

    // Must be <= 1,000,000 bytes uncompressed
    assert!(
        index_json.len() <= 1_000_000,
        "Index size {} exceeds 1,000,000 bytes limit",
        index_json.len()
    );

    println!(
        "Synthetic 1,000-object index size: {} bytes ({:.2} KB)",
        index_json.len(),
        index_json.len() as f64 / 1024.0
    );
}
