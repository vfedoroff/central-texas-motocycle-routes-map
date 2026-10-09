mod common;
use anyhow::Result;
use catalog_build::{BuildConfig, build_site, validate_build_paths};
use std::fs;
use tempfile::tempdir;

fn setup_fixture() -> Result<tempfile::TempDir> {
    let dir = tempdir()?;
    let root = dir.path();

    // Cargo.toml
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test-fixture"
version = "0.1.0"
edition = "2024"
"#,
    )?;

    // Content directories
    fs::create_dir_all(root.join("content/routes"))?;
    fs::create_dir_all(root.join("content/geometry/routes"))?;

    // Route GeoJSON
    let geojson = r#"{
  "type": "Feature",
  "properties": {},
  "geometry": {
    "type": "LineString",
    "coordinates": [
      [-98.0, 30.0],
      [-98.0, 30.1],
      [-98.1, 30.1],
      [-98.0, 30.0]
    ]
  }
}"#;
    fs::write(
        root.join("content/geometry/routes/test-route.geojson"),
        geojson,
    )?;

    // Route record
    let route = r##"{
  "schema_version": 1,
  "id": "test-route",
  "title": "Test Route",
  "summary": "A loop route for testing.",
  "category": "Twisties & Canyons",
  "color": "#e74c3c",
  "distance_mi": 25.0,
  "waypoints": ["Start", "Point B", "Point C", "Start"],
  "via": "Road A -> Road B",
  "geometry_path": "content/geometry/routes/test-route.geojson",
  "route_type": "loop",
  "tags": ["test"],
  "sources": []
}"##;
    fs::write(root.join("content/routes/test-route.json"), route)?;

    common::seed_map_cache(root)?;
    Ok(dir)
}

#[test]
fn test_successful_dev_build() -> Result<()> {
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

    assert!(out.join("catalog-index.json").exists());
    assert!(out.join("data/routes/test-route.json").exists());
    assert!(out.join("data/geometry/routes/test-route.geojson").exists());
    assert!(out.join("data/overview/routes/test-route.geojson").exists());

    let html = fs::read_to_string(out.join("routes/test-route/index.html"))?;
    assert!(html.contains("property=\"og:title\" content=\"Test Route\""));
    assert!(html.contains("name=\"twitter:card\" content=\"summary_large_image\""));
    assert!(html.contains("/social/routes/test-route.png"));
    assert!(html.contains("<img class=\"route-preview\" src=\"/social/routes/test-route.png\""));
    let preview = image::open(out.join("social/routes/test-route.png"))?;
    assert_eq!((preview.width(), preview.height()), (1200, 630));

    // Verify index content
    let index_text = fs::read_to_string(out.join("catalog-index.json"))?;
    assert!(index_text.contains("test-route"));

    // Verify .git and source configs are absent
    assert!(!out.join(".git").exists());
    assert!(!out.join("Cargo.toml").exists());

    Ok(())
}

#[test]
fn test_failed_validation_preserves_sentinel() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");

    // Create existing output directory with a sentinel file
    fs::create_dir_all(&out)?;
    let sentinel = out.join("sentinel.txt");
    fs::write(&sentinel, "original content")?;

    // Corrupt the route file so validation will fail
    fs::write(
        root.join("content/routes/test-route.json"),
        r#"{"schema_version": 1, "id": "test-route"}"#, // Missing required fields
    )?;

    let cfg = BuildConfig {
        root: root.clone(),
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: None,
    };

    let result = build_site(&cfg);
    assert!(result.is_err());

    // Sentinel must still be intact
    assert!(sentinel.exists());
    assert_eq!(fs::read_to_string(&sentinel)?, "original content");

    Ok(())
}

#[test]
fn test_invalid_output_paths_rejected() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path();

    // 1. Output identical to root
    let err = validate_build_paths(root, root);
    assert!(err.is_err());

    // 2. Output inside content directory
    let content_dir = root.join("content");
    let err = validate_build_paths(root, &content_dir);
    assert!(err.is_err());

    let content_sub = root.join("content/nested_out");
    let err = validate_build_paths(root, &content_sub);
    assert!(err.is_err());

    // 3. Output inside crates directory
    let crates_sub = root.join("crates/nested_out");
    let err = validate_build_paths(root, &crates_sub);
    assert!(err.is_err());

    // 4. Missing Cargo.toml in root
    let empty_dir = tempdir()?;
    let err = validate_build_paths(empty_dir.path(), &empty_dir.path().join("dist"));
    assert!(err.is_err());

    Ok(())
}

#[test]
fn test_symlink_output_rejected() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path();

    let real_dir = root.join("some_real_dir");
    fs::create_dir_all(&real_dir)?;

    let symlink_out = root.join("symlink_dist");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_dir, &symlink_out)?;

    #[cfg(unix)]
    {
        let err = validate_build_paths(root, &symlink_out);
        assert!(err.is_err());
    }

    Ok(())
}

#[test]
fn test_successful_ui_composition_and_asset_verification() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");
    let ui_dir = root.join(".build/ui");
    fs::create_dir_all(&ui_dir)?;

    // Create mock Trunk index.html
    let trunk_index = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>Trunk Build</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <script type="module">import init from '/app/catalog-ui-mock123.js';</script>
    <link rel="stylesheet" href="/app/style-mock123.css" />
    <link rel="preload" href="/app/catalog-ui-mock123_bg.wasm" as="fetch" type="application/wasm" />
</head>
<body></body>
</html>"#;
    fs::write(ui_dir.join("index.html"), trunk_index)?;

    // Create the referenced files in ui_dir
    fs::write(ui_dir.join("catalog-ui-mock123.js"), "console.log('wasm');")?;
    fs::write(ui_dir.join("style-mock123.css"), "body { margin: 0; }")?;
    fs::write(
        ui_dir.join("catalog-ui-mock123_bg.wasm"),
        b"\x00asm\x01\x00\x00\x00",
    )?;

    let cfg = BuildConfig {
        root: root.clone(),
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: Some(ui_dir),
    };

    build_site(&cfg)?;

    // Root map index.html rendered
    assert!(out.join("index.html").exists());
    let index_html = fs::read_to_string(out.join("index.html"))?;
    assert!(index_html.contains("Ride Atlas — Central Texas Motorcycle Routes"));
    assert!(index_html.contains("/app/catalog-ui-mock123.js"));
    assert!(index_html.contains("/app/style-mock123.css"));

    // UI assets copied into dist/app
    assert!(out.join("app/catalog-ui-mock123.js").exists());
    assert!(out.join("app/style-mock123.css").exists());
    assert!(out.join("app/catalog-ui-mock123_bg.wasm").exists());
    assert!(!out.join("app/index.html").exists());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("app/manifest.json"))?)?;
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(
        manifest["scripts"],
        serde_json::json!(["/app/catalog-ui-mock123.js"])
    );
    assert_eq!(
        manifest["css"],
        serde_json::json!(["/app/style-mock123.css"])
    );
    assert_eq!(
        manifest["wasm"],
        serde_json::json!(["/app/catalog-ui-mock123_bg.wasm"])
    );
    assert!(!index_html.contains("unverified route"));
    let page = fs::read_to_string(out.join("routes/test-route/index.html"))?;
    assert!(!page.contains("unverified route"));
    assert!(
        page.contains("https://central-texas-routes-map.netlify.app/routes/test-route/index.html")
    );

    // Static route pages intact
    assert!(out.join("routes/test-route/index.html").exists());

    Ok(())
}

#[test]
fn test_ui_composition_rejects_unsafe_reference_and_preserves_output() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");
    fs::create_dir_all(&out)?;
    fs::write(out.join("sentinel.txt"), "previous build")?;
    let ui_dir = root.join(".build/ui");
    fs::create_dir_all(&ui_dir)?;
    fs::write(
        ui_dir.join("index.html"),
        "<html><head><script src=\"/app/../secret.js\"></script></head></html>",
    )?;
    let cfg = BuildConfig {
        root,
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: Some(ui_dir),
    };
    let error = build_site(&cfg).unwrap_err().to_string();
    assert!(error.contains("Unsafe UI asset URL"));
    assert_eq!(
        fs::read_to_string(out.join("sentinel.txt"))?,
        "previous build"
    );
    Ok(())
}

#[test]
fn test_ui_composition_rejects_unstaged_asset_attribute() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let ui_dir = root.join(".build/ui");
    fs::create_dir_all(&ui_dir)?;
    fs::write(
        ui_dir.join("index.html"),
        "<html><head><script src=\"https://example.com/app.js\"></script><link href=\"/app/style.css\"></head></html>",
    )?;
    let cfg = BuildConfig {
        root: root.clone(),
        out: root.join("dist"),

        environment: "development".to_string(),

        ui_dir: Some(ui_dir),
    };
    assert!(
        build_site(&cfg)
            .unwrap_err()
            .to_string()
            .contains("UI asset URL must begin with /app/")
    );
    Ok(())
}

#[test]
fn test_ui_composition_rejects_external_ui_directory() -> Result<()> {
    let dir = setup_fixture()?;
    let external = tempdir()?;
    let root = dir.path().to_path_buf();
    fs::write(
        external.path().join("index.html"),
        "<html><head></head></html>",
    )?;
    let cfg = BuildConfig {
        root: root.clone(),
        out: root.join("dist"),

        environment: "development".to_string(),

        ui_dir: Some(external.path().to_path_buf()),
    };
    assert!(
        build_site(&cfg)
            .unwrap_err()
            .to_string()
            .contains("inside the build root")
    );
    Ok(())
}

#[test]
fn test_ui_composition_fails_on_missing_referenced_asset() -> Result<()> {
    let dir = setup_fixture()?;
    let root = dir.path().to_path_buf();
    let out = root.join("dist");
    let ui_dir = root.join(".build/ui");
    fs::create_dir_all(&ui_dir)?;

    // References missing file that does not exist in ui_dir
    let trunk_index = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <link rel="stylesheet" href="/app/nonexistent-file.css" />
</head>
<body></body>
</html>"#;
    fs::write(ui_dir.join("index.html"), trunk_index)?;

    let cfg = BuildConfig {
        root: root.clone(),
        out: out.clone(),

        environment: "development".to_string(),

        ui_dir: Some(ui_dir),
    };

    let result = build_site(&cfg);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Referenced UI asset URL does not exist on disk"));

    Ok(())
}

#[test]
fn production_publishes_catalog() -> Result<()> {
    let dir = setup_fixture()?;
    let cfg = BuildConfig {
        root: dir.path().to_path_buf(),
        out: dir.path().join("dist"),

        environment: "production".into(),

        ui_dir: None,
    };
    build_site(&cfg)?;
    let html = fs::read_to_string(cfg.out.join("routes/test-route/index.html"))?;
    assert!(html.contains("Test Route"));
    assert!(cfg.out.join("catalog-index.json").exists());
    Ok(())
}
