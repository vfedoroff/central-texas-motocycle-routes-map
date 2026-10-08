use catalog_build::pwa::{
    compute_site_version, generate_brand_icon, generate_manifest, generate_pwa_icons,
    generate_service_worker,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_brand_icon_generation() {
    let icon_192 = generate_brand_icon(192, false).expect("192 icon generation failed");
    assert!(!icon_192.is_empty());
    // Verify valid PNG header (bytes 0..8: 137, 80, 78, 71, 13, 10, 26, 10)
    assert_eq!(&icon_192[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

    let icon_512 = generate_brand_icon(512, false).expect("512 icon generation failed");
    assert_eq!(&icon_512[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

    let maskable_192 = generate_brand_icon(192, true).expect("maskable 192 icon generation failed");
    assert_eq!(&maskable_192[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
}

#[test]
fn test_manifest_and_icons_generation() {
    let dir = tempdir().expect("tempdir");
    let staging = dir.path();

    generate_pwa_icons(staging).expect("generate icons");
    assert!(staging.join("icons/icon-192.png").exists());
    assert!(staging.join("icons/icon-512.png").exists());
    assert!(staging.join("icons/icon-maskable-192.png").exists());
    assert!(staging.join("icons/icon-maskable-512.png").exists());

    generate_manifest(staging, "Ride Atlas Test").expect("generate manifest");
    let manifest_file = staging.join("manifest.webmanifest");
    assert!(manifest_file.exists());

    let content = fs::read_to_string(&manifest_file).expect("read manifest");
    let val: serde_json::Value = serde_json::from_str(&content).expect("parse json");
    assert_eq!(val["name"], "Ride Atlas Test");
    assert_eq!(val["display"], "standalone");
    assert_eq!(val["start_url"], "/");
    assert_eq!(val["theme_color"], "#1e293b");

    let icons = val["icons"].as_array().expect("icons array");
    assert_eq!(icons.len(), 4);
}

#[test]
fn test_site_version_and_service_worker() {
    let dir = tempdir().expect("tempdir");
    let staging = dir.path();

    fs::write(
        staging.join("index.html"),
        "<html><body>Ride Atlas</body></html>",
    )
    .unwrap();
    fs::write(staging.join("test.txt"), "hello world").unwrap();

    let site_ver1 = compute_site_version(staging).expect("compute site version");
    assert_eq!(site_ver1.len(), 64); // SHA-256 hex string

    // Noncircular check: presence of service-worker.js or offline-manifest.json does not change version
    fs::write(staging.join("service-worker.js"), "// dummy worker").unwrap();
    fs::write(staging.join("offline-manifest.json"), "{}").unwrap();

    let site_ver2 = compute_site_version(staging).expect("compute site version after sw");
    assert_eq!(site_ver1, site_ver2);

    let shell_urls = vec!["/".to_string(), "/index.html".to_string()];
    generate_service_worker(staging, &site_ver1, &shell_urls).expect("generate sw");

    let sw_content = fs::read_to_string(staging.join("service-worker.js")).expect("read sw");
    assert!(sw_content.contains(&site_ver1));
    assert!(sw_content.contains("SHELL_CACHE"));
    assert!(sw_content.contains("SKIP_WAITING"));
    assert!(sw_content.contains("openstreetmap"));
}
