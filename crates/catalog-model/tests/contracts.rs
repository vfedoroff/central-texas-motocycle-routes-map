use catalog_model::{AuthoredPlace, AuthoredRoad, Diagnostic, ObjectKey, ObjectKind, Severity};

#[test]
fn object_key_roundtrip() {
    let key = ObjectKey {
        kind: ObjectKind::Place,
        id: "fixture-overlook".into(),
    };
    let json = serde_json::to_string(&key).unwrap();
    assert_eq!(json, r#"{"kind":"place","id":"fixture-overlook"}"#);
    assert_eq!(serde_json::from_str::<ObjectKey>(&json).unwrap(), key);
}

#[test]
fn unknown_field_rejected() {
    let valid = r#"{"schema_version":1,"id":"test","title":"Test","summary":"Test summary","place_category":"viewpoint","coordinates":[-97.8,30.3]}"#;
    assert!(serde_json::from_str::<AuthoredPlace>(valid).is_ok());
    let unknown = valid.replace("\"coordinates\":", "\"surprise\":true,\"coordinates\":");
    assert!(serde_json::from_str::<AuthoredPlace>(&unknown).is_err());

    let with_related = valid.replace(
        "\"coordinates\":",
        "\"related\":[{\"kind\":\"place\",\"id\":\"lookout\"}],\"coordinates\":",
    );
    assert!(serde_json::from_str::<AuthoredPlace>(&with_related).is_ok());
    let nested_unknown =
        with_related.replace("\"id\":\"lookout\"", "\"id\":\"lookout\",\"surprise\":true");
    assert!(serde_json::from_str::<AuthoredPlace>(&nested_unknown).is_err());
}

#[test]
fn place_requires_coordinates() {
    let valid = r#"{"schema_version":1,"id":"test","title":"Test","summary":"Test summary","place_category":"viewpoint","coordinates":[-97.8,30.3]}"#;
    assert!(serde_json::from_str::<AuthoredPlace>(valid).is_ok());
    let json = r#"{"schema_version":1,"id":"test","title":"Test","summary":"Test summary","place_category":"viewpoint"}"#;
    assert!(serde_json::from_str::<AuthoredPlace>(json).is_err());
}

#[test]
fn road_requires_surface_evidence() {
    let valid = r#"{"schema_version":1,"id":"test","title":"Test","summary":"Test summary","geometry_path":"content/geometry/roads/test.geojson","surface":"paved","surface_evidence":[{"title":"Road office","url":"https://example.org/paving","accessed_on":"2026-10-07"}]}"#;
    assert!(serde_json::from_str::<AuthoredRoad>(valid).is_ok());
    let empty = valid.replace(r#"[{"title":"Road office","url":"https://example.org/paving","accessed_on":"2026-10-07"}]"#, "[]");
    assert!(serde_json::from_str::<AuthoredRoad>(&empty).is_err());
}

#[test]
fn pinned_rust_versions_match() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let toolchain = std::fs::read_to_string(root.join("rust-toolchain.toml")).unwrap();
    let mise = std::fs::read_to_string(root.join("mise.toml")).unwrap();
    let version = toolchain
        .lines()
        .find_map(|line| line.trim().strip_prefix("channel = "))
        .unwrap();
    assert!(
        mise.lines()
            .any(|line| line.trim() == format!("rust = {version}"))
    );
}

#[test]
fn diagnostic_optional_fields_have_fixed_null_shape() {
    let diagnostic = Diagnostic {
        code: "fixture".into(),
        severity: Severity::Review,
        key: None,
        file: "content/roads/test.json".into(),
        json_pointer: None,
        coordinate_range: None,
        edge_id: None,
        message: "Review required".into(),
    };
    let json = serde_json::to_value(diagnostic).unwrap();
    for field in ["key", "json_pointer", "coordinate_range", "edge_id"] {
        assert!(json.get(field).unwrap().is_null(), "{field}");
    }
}
