use crate::{
    AuthoredPlace, AuthoredRoad, AuthoredRoute, Catalog, CatalogObject, CatalogPayload, Coordinate,
    Diagnostic, ObjectKey, ObjectKind, Severity,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

fn diagnostic(
    code: &str,
    file: &str,
    key: Option<ObjectKey>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        key,
        file: file.into(),
        json_pointer: None,
        coordinate_range: None,
        edge_id: None,
        message: message.into(),
    }
}

fn read_record<T: DeserializeOwned>(
    path: &Path,
    display: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<T> {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(record) => Some(record),
            Err(err) => {
                errors.push(diagnostic("invalid_record", display, None, err.to_string()));
                None
            }
        },
        Err(err) => {
            errors.push(diagnostic(
                "unreadable_record",
                display,
                None,
                err.to_string(),
            ));
            None
        }
    }
}

fn content_files(root: &Path, kind: &str, errors: &mut Vec<Diagnostic>) -> Vec<PathBuf> {
    let dir = root.join("content").join(kind);
    if !dir.exists() {
        return Vec::new();
    }
    let mut files = Vec::new();
    match fs::read_dir(&dir) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.extension().is_some_and(|s| s == "json") && path.is_file() {
                            files.push(path);
                        } else {
                            errors.push(diagnostic(
                                "unknown_content_file",
                                &path
                                    .strip_prefix(root)
                                    .unwrap_or(&path)
                                    .display()
                                    .to_string(),
                                None,
                                "expected a JSON record",
                            ));
                        }
                    }
                    Err(err) => errors.push(diagnostic(
                        "unreadable_directory",
                        &format!("content/{kind}"),
                        None,
                        err.to_string(),
                    )),
                }
            }
        }
        Err(err) => errors.push(diagnostic(
            "unreadable_directory",
            &format!("content/{kind}"),
            None,
            err.to_string(),
        )),
    }
    files.sort();
    files
}

fn geometry(
    root: &Path,
    kind: &str,
    object: &CatalogObject,
    path: &str,
    errors: &mut Vec<Diagnostic>,
) -> Option<Vec<Coordinate>> {
    let expected = format!("content/geometry/{kind}/{}.geojson", object.key.id);
    let file = format!("content/{kind}/{}.json", object.key.id);
    if path != expected
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        errors.push(diagnostic(
            "invalid_geometry_path",
            &file,
            Some(object.key.clone()),
            format!("expected {expected}"),
        ));
        return None;
    }
    let source = root.join(path);
    if !source.exists() {
        errors.push(diagnostic(
            "missing_geometry",
            path,
            Some(object.key.clone()),
            "geometry file is missing",
        ));
        return None;
    }
    let base = match root.join("content/geometry").canonicalize() {
        Ok(p) => p,
        Err(err) => {
            errors.push(diagnostic(
                "unreadable_geometry",
                path,
                Some(object.key.clone()),
                err.to_string(),
            ));
            return None;
        }
    };
    if !source.canonicalize().is_ok_and(|p| p.starts_with(&base)) {
        errors.push(diagnostic(
            "invalid_geometry_path",
            path,
            Some(object.key.clone()),
            "geometry escapes content/geometry",
        ));
        return None;
    }
    let value: Value = read_record(&source, path, errors)?;
    let valid_shape = value.get("type").and_then(Value::as_str) == Some("Feature")
        && value
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|p| p.is_empty())
        && value
            .get("geometry")
            .and_then(|g| g.get("type"))
            .and_then(Value::as_str)
            == Some("LineString");
    if !valid_shape {
        errors.push(diagnostic(
            "invalid_geometry",
            path,
            Some(object.key.clone()),
            "expected one LineString Feature with empty properties",
        ));
        return None;
    }
    let Some(raw) = value
        .pointer("/geometry/coordinates")
        .and_then(Value::as_array)
    else {
        errors.push(diagnostic(
            "invalid_geometry",
            path,
            Some(object.key.clone()),
            "coordinates must be an array",
        ));
        return None;
    };
    let mut coordinates = Vec::with_capacity(raw.len());
    for point in raw {
        let Some(pair) = point.as_array().filter(|p| p.len() == 2) else {
            errors.push(diagnostic(
                "invalid_coordinate",
                path,
                Some(object.key.clone()),
                "coordinate must have exactly two values",
            ));
            return None;
        };
        let (Some(lon), Some(lat)) = (pair[0].as_f64(), pair[1].as_f64()) else {
            errors.push(diagnostic(
                "invalid_coordinate",
                path,
                Some(object.key.clone()),
                "coordinate must be numeric",
            ));
            return None;
        };
        if !lon.is_finite()
            || !lat.is_finite()
            || !(-180.0..=180.0).contains(&lon)
            || !(-90.0..=90.0).contains(&lat)
        {
            errors.push(diagnostic(
                "invalid_coordinate",
                path,
                Some(object.key.clone()),
                "coordinate out of range",
            ));
            return None;
        }
        let point = [lon, lat];
        if coordinates.last() == Some(&point) {
            errors.push(diagnostic(
                "adjacent_duplicate",
                path,
                Some(object.key.clone()),
                "adjacent coordinate is duplicated",
            ));
            return None;
        }
        coordinates.push(point);
    }
    if coordinates.len() < 2 || coordinates.iter().all(|p| *p == coordinates[0]) {
        errors.push(diagnostic(
            "invalid_geometry",
            path,
            Some(object.key.clone()),
            "at least two distinct coordinates required",
        ));
        return None;
    }
    Some(coordinates)
}

pub fn load_catalog(root: &Path) -> Result<Catalog, Vec<Diagnostic>> {
    let mut objects = Vec::new();
    let mut errors = Vec::new();
    let mut keys = HashSet::new();
    let mut used_geometry = HashSet::new();
    for (directory, kind) in [
        ("routes", ObjectKind::Route),
        ("roads", ObjectKind::Road),
        ("places", ObjectKind::Place),
    ] {
        for path in content_files(root, directory, &mut errors) {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let object: Option<CatalogObject> = match kind {
                ObjectKind::Route => {
                    read_record::<AuthoredRoute>(&path, &relative, &mut errors).map(Into::into)
                }
                ObjectKind::Road => {
                    read_record::<AuthoredRoad>(&path, &relative, &mut errors).map(Into::into)
                }
                ObjectKind::Place => {
                    read_record::<AuthoredPlace>(&path, &relative, &mut errors).map(Into::into)
                }
            };
            let Some(mut object) = object else { continue };
            object.source_path = relative.clone();
            let key = object.key.clone();
            if !keys.insert(key.clone()) {
                errors.push(diagnostic(
                    "duplicate_key",
                    &relative,
                    Some(key),
                    "duplicate kind/id",
                ));
                continue;
            }
            if path.file_stem().and_then(|s| s.to_str()) != Some(object.key.id.as_str())
                || !valid_id(&object.key.id)
            {
                errors.push(diagnostic(
                    "invalid_id",
                    &relative,
                    Some(key.clone()),
                    "id must be kebab-case and match filename",
                ));
            }
            if object.schema_version != 1 {
                errors.push(diagnostic(
                    "unsupported_schema",
                    &relative,
                    Some(key.clone()),
                    "schema_version must be 1",
                ));
            }
            if object.title.trim().is_empty()
                || object.summary.trim().is_empty()
                || object.summary.chars().count() > 280
            {
                errors.push(diagnostic(
                    "invalid_text",
                    &relative,
                    Some(key.clone()),
                    "title and summary must be nonempty; summary maximum is 280 characters",
                ));
            }
            let unique: HashSet<_> = object.tags.iter().collect();
            if unique.len() != object.tags.len() {
                errors.push(diagnostic(
                    "duplicate_tag",
                    &relative,
                    Some(key.clone()),
                    "duplicate tags",
                ));
            }
            let geometry_path = match &object.payload {
                CatalogPayload::Route { geometry_path, .. }
                | CatalogPayload::Road { geometry_path, .. } => Some(geometry_path.clone()),
                CatalogPayload::Place { .. } => None,
            };
            if let Some(path) = geometry_path {
                used_geometry.insert(path.clone());
                object.geometry = geometry(root, directory, &object, &path, &mut errors);
            }
            objects.push(object);
        }
    }
    for kind in ["routes", "roads"] {
        let dir = root.join("content/geometry").join(kind);
        if !dir.exists() {
            continue;
        }
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let relative = path
                                .strip_prefix(root)
                                .unwrap_or(&path)
                                .display()
                                .to_string();
                            if !used_geometry.contains(&relative) {
                                errors.push(diagnostic(
                                    "unknown_geometry_file",
                                    &relative,
                                    None,
                                    "geometry has no matching authored record",
                                ));
                            }
                        }
                        Err(err) => errors.push(diagnostic(
                            "unreadable_directory",
                            &format!("content/geometry/{kind}"),
                            None,
                            err.to_string(),
                        )),
                    }
                }
            }
            Err(err) => errors.push(diagnostic(
                "unreadable_directory",
                &format!("content/geometry/{kind}"),
                None,
                err.to_string(),
            )),
        }
    }
    if objects.is_empty() && errors.is_empty() {
        errors.push(diagnostic(
            "missing_content",
            "content",
            None,
            "no new-schema content records found",
        ));
    }
    errors.sort_by(|a, b| {
        (
            &a.file,
            format!("{:?}", a.key),
            &a.code,
            a.coordinate_range,
            &a.message,
        )
            .cmp(&(
                &b.file,
                format!("{:?}", b.key),
                &b.code,
                b.coordinate_range,
                &b.message,
            ))
    });
    if errors.is_empty() {
        objects.sort_by(|a, b| {
            format!("{:?}/{}", a.key.kind, a.key.id).cmp(&format!("{:?}/{}", b.key.kind, b.key.id))
        });
        Ok(Catalog { objects })
    } else {
        Err(errors)
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
