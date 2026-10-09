#![cfg(feature = "native")]

use catalog_model::{CatalogPayload, RouteType, load_catalog};
use std::{fs, path::Path};

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn route(root: &Path, filename: &str, id: &str, geometry: bool) {
    fs::create_dir_all(root.join("content/routes")).unwrap();
    fs::write(root.join("content/routes").join(filename), format!(r##"{{"schema_version":1,"id":"{id}","title":"Test route","summary":"A route","category":"Twisties & Canyons","color":"#e74c3c","distance_mi":6.9,"waypoints":["A","B"],"via":"A to B","geometry_path":"content/geometry/routes/{id}.geojson","route_type":"corridor"}}"##)).unwrap();
    if geometry {
        fs::create_dir_all(root.join("content/geometry/routes")).unwrap();
        fs::write(root.join(format!("content/geometry/routes/{id}.geojson")), r#"{"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[-98.0,30.0],[-98.0,30.1]]}}"#).unwrap();
    }
}

#[test]
fn all_migrated_routes_load() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = load_catalog(&root).unwrap();
    let authored_routes = fs::read_dir(root.join("content/routes"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .count();
    let loaded_routes = catalog
        .objects
        .iter()
        .filter(|o| matches!(o.payload, CatalogPayload::Route { .. }))
        .count();
    assert!(
        authored_routes >= 26,
        "original migrated routes remain present"
    );
    assert_eq!(loaded_routes, authored_routes);
    assert!(catalog.objects.iter().any(|o| matches!(
        o.payload,
        CatalogPayload::Route {
            route_type: RouteType::Corridor,
            ..
        }
    )));
}

#[test]
fn duplicate_route_id_rejected() {
    let tmp = fixture();
    route(tmp.path(), "a.json", "same", true);
    route(tmp.path(), "b.json", "same", true);
    let errors = load_catalog(tmp.path()).unwrap_err();
    assert!(errors.iter().any(|d| d.code == "duplicate_key"));
}

#[test]
fn missing_geometry_rejected() {
    let tmp = fixture();
    route(tmp.path(), "route.json", "missing", false);
    let errors = load_catalog(tmp.path()).unwrap_err();
    assert!(errors.iter().any(|d| d.code == "missing_geometry"));
}

#[test]
fn new_schema_only() {
    let tmp = fixture();
    fs::write(tmp.path().join("routes.json"), "[]").unwrap();
    assert!(
        load_catalog(tmp.path())
            .unwrap_err()
            .iter()
            .any(|d| d.code == "missing_content")
    );
}

#[test]
fn geometry_path_escape_rejected() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", false);
    let file = tmp.path().join("content/routes/safe.json");
    let text = fs::read_to_string(&file).unwrap().replace(
        "content/geometry/routes/safe.geojson",
        "../routes/safe.geojson",
    );
    fs::write(file, text).unwrap();
    assert!(
        load_catalog(tmp.path())
            .unwrap_err()
            .iter()
            .any(|d| d.code == "invalid_geometry_path")
    );
}

#[test]
fn unknown_content_file_rejected() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", true);
    fs::write(tmp.path().join("content/routes/note.txt"), "surprise").unwrap();
    assert!(
        load_catalog(tmp.path())
            .unwrap_err()
            .iter()
            .any(|d| d.code == "unknown_content_file")
    );
}

#[test]
fn orphan_geometry_file_rejected() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", true);
    fs::write(
        tmp.path().join("content/geometry/routes/orphan.geojson"),
        "{}",
    )
    .unwrap();
    assert!(
        load_catalog(tmp.path())
            .unwrap_err()
            .iter()
            .any(|d| d.code == "unknown_geometry_file")
    );
}

#[test]
fn malformed_geometry_rejected() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", true);
    fs::write(
        tmp.path().join("content/geometry/routes/safe.geojson"),
        r#"{"type":"FeatureCollection","features":[]}"#,
    )
    .unwrap();
    assert!(
        load_catalog(tmp.path())
            .unwrap_err()
            .iter()
            .any(|d| d.code == "invalid_geometry")
    );
}

#[test]
fn unsupported_schema_reports_source_file() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", true);
    let file = tmp.path().join("content/routes/safe.json");
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("\"schema_version\":1", "\"schema_version\":2"),
    )
    .unwrap();
    let errors = load_catalog(tmp.path()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|d| d.code == "unsupported_schema" && d.file == "content/routes/safe.json")
    );
}

#[test]
fn loaded_object_retains_authored_provenance() {
    let tmp = fixture();
    route(tmp.path(), "safe.json", "safe", true);
    let catalog = load_catalog(tmp.path()).unwrap();
    assert_eq!(catalog.objects[0].source_path, "content/routes/safe.json");
    assert_eq!(catalog.objects[0].schema_version, 1);
}
