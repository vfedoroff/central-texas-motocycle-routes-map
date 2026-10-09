mod common;
use anyhow::Result;
use catalog_build::{BuildConfig, build_site};
use catalog_model::ObjectDetail;
use std::fs;
use tempfile::tempdir;

fn setup_fixture() -> Result<tempfile::TempDir> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test-fixture"
version = "0.1.0"
edition = "2024"
"#,
    )?;

    fs::create_dir_all(root.join("content/routes"))?;
    fs::create_dir_all(root.join("content/geometry/routes"))?;

    let geojson = r#"{
  "type": "Feature",
  "properties": {},
  "geometry": {
    "type": "LineString",
    "coordinates": [
      [-98.12345, 30.54321],
      [-98.15000, 30.56000],
      [-98.16000, 30.58000],
      [-98.12345, 30.54321]
    ]
  }
}"#;
    fs::write(
        root.join("content/geometry/routes/sample-loop.geojson"),
        geojson,
    )?;

    let route = r##"{
  "schema_version": 1,
  "id": "sample-loop",
  "title": "Sample Loop & Ride",
  "summary": "Scenic test loop.",
  "category": "Twisties & Canyons",
  "color": "#e74c3c",
  "distance_mi": 18.5,
  "waypoints": ["Start", "Turn", "Start"],
  "via": "Scenic Way",
  "geometry_path": "content/geometry/routes/sample-loop.geojson",
  "route_type": "loop",
  "tags": ["scenic"],
  "sources": []
}"##;
    fs::write(root.join("content/routes/sample-loop.json"), route)?;

    common::seed_map_cache(root)?;
    Ok(dir)
}

fn parse_gpx_points(gpx_xml: &str) -> Vec<[f64; 2]> {
    let mut points = Vec::new();
    for line in gpx_xml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<trkpt ") {
            // e.g. <trkpt lat="30.54321" lon="-98.12345"/>
            let lat_start = trimmed.find("lat=\"").unwrap() + 5;
            let lat_end = trimmed[lat_start..].find('"').unwrap() + lat_start;
            let lat: f64 = trimmed[lat_start..lat_end].parse().unwrap();

            let lon_start = trimmed.find("lon=\"").unwrap() + 5;
            let lon_end = trimmed[lon_start..].find('"').unwrap() + lon_start;
            let lon: f64 = trimmed[lon_start..lon_end].parse().unwrap();

            points.push([lon, lat]);
        }
    }
    points
}

#[test]
fn test_gpx_coordinates_match_geometry() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");

    let cfg = BuildConfig {
        root: root.clone(),
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg)?;

    let gpx_file = out.join("downloads/routes/sample-loop.gpx");
    assert!(gpx_file.exists());
    let gpx_content = fs::read_to_string(&gpx_file)?;

    let parsed_coords = parse_gpx_points(&gpx_content);
    let original_coords = [
        [-98.12345, 30.54321],
        [-98.15000, 30.56000],
        [-98.16000, 30.58000],
        [-98.12345, 30.54321],
    ];

    assert_eq!(parsed_coords.len(), original_coords.len());
    for (actual, expected) in parsed_coords.iter().zip(original_coords.iter()) {
        assert!((actual[0] - expected[0]).abs() < 1e-6);
        assert!((actual[1] - expected[1]).abs() < 1e-6);
    }

    Ok(())
}

#[test]
fn test_gpx_namespace_valid() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");

    let cfg = BuildConfig {
        root,
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg)?;

    let gpx_file = out.join("downloads/routes/sample-loop.gpx");
    let gpx_content = fs::read_to_string(gpx_file)?;

    assert!(gpx_content.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(gpx_content.contains("version=\"1.1\""));
    assert!(gpx_content.contains("creator=\"Ride Atlas\""));
    assert!(gpx_content.contains("xmlns=\"http://www.topografix.com/GPX/1/1\""));
    // Title with & escaped
    assert!(gpx_content.contains("<name>Sample Loop &amp; Ride</name>"));

    Ok(())
}

#[test]
fn test_download_links_resolve() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");

    let cfg = BuildConfig {
        root,
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg)?;

    let detail_file = out.join("data/routes/sample-loop.json");
    assert!(detail_file.exists());
    let detail: ObjectDetail = serde_json::from_str(&fs::read_to_string(detail_file)?)?;

    // detail.gpx_url must point to a file that actually exists under dist
    let gpx_url = detail.gpx_url.expect("gpx_url should be set");
    assert!(gpx_url.starts_with('/'));
    let gpx_disk_path = out.join(gpx_url.trim_start_matches('/'));
    assert!(gpx_disk_path.exists());

    // GeoJSON download must also exist
    let geojson_download = out.join("downloads/routes/sample-loop.geojson");
    assert!(geojson_download.exists());

    // Verify parsed GeoJSON download coordinates
    let geojson_val: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(geojson_download)?)?;
    assert_eq!(geojson_val["type"], "Feature");
    assert_eq!(geojson_val["geometry"]["type"], "LineString");

    Ok(())
}

#[test]
fn test_no_old_bundles_generated() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");

    let cfg = BuildConfig {
        root,
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    build_site(&cfg)?;

    assert!(!out.join("routes.zip").exists());
    assert!(!out.join("all-routes.gpx").exists());
    assert!(!out.join("routes.json").exists());
    assert!(!out.join("bundle.json").exists());

    Ok(())
}
