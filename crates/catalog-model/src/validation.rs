use crate::{
    Catalog, CatalogObject, CatalogPayload, Coordinate, Diagnostic, ObjectKey, ObjectKind,
    RouteType, Severity, VisitStatus, topology::detect_spurs,
};
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path},
};

fn diagnostic(
    code: &str,
    file: &str,
    key: Option<ObjectKey>,
    coordinate_range: Option<[usize; 2]>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        key,
        file: file.into(),
        json_pointer: None,
        coordinate_range,
        edge_id: None,
        message: message.into(),
    }
}

pub fn is_valid_http_url(raw: &str) -> bool {
    let raw = raw.trim();
    if !raw.starts_with("http://") && !raw.starts_with("https://") {
        return false;
    }
    // Basic structural check without requiring heavy crates
    if let Some(rest) = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))
    {
        let host = rest.split('/').next().unwrap_or("");
        !host.is_empty() && !host.contains(' ')
    } else {
        false
    }
}

pub fn has_path_escape(path_str: &str) -> bool {
    let p = Path::new(path_str);
    p.is_absolute()
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
        || path_str.contains("..")
}

pub fn validate_catalog(catalog: &Catalog) -> Vec<Diagnostic> {
    validate_catalog_with_options(catalog, false)
}

pub fn validate_catalog_with_options(catalog: &Catalog, audit_spurs: bool) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut known_keys: HashSet<ObjectKey> = HashSet::new();
    let mut object_map: HashMap<ObjectKey, &CatalogObject> = HashMap::new();

    // First pass: index objects and check uniqueness
    for obj in &catalog.objects {
        if !known_keys.insert(obj.key.clone()) {
            diagnostics.push(diagnostic(
                "duplicate_key",
                &obj.source_path,
                Some(obj.key.clone()),
                None,
                format!("duplicate key {:?}:{}", obj.key.kind, obj.key.id),
            ));
        } else {
            object_map.insert(obj.key.clone(), obj);
        }

        // Filename / ID mismatch
        if !obj.source_path.is_empty() {
            let path = Path::new(&obj.source_path);
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem != obj.key.id
            {
                diagnostics.push(diagnostic(
                    "filename_id_mismatch",
                    &obj.source_path,
                    Some(obj.key.clone()),
                    None,
                    format!(
                        "filename stem '{}' does not match id '{}'",
                        stem, obj.key.id
                    ),
                ));
            }
        }

        // Duplicate tags
        let mut seen_tags = HashSet::new();
        for tag in &obj.tags {
            if !seen_tags.insert(tag) {
                diagnostics.push(diagnostic(
                    "duplicate_tag",
                    &obj.source_path,
                    Some(obj.key.clone()),
                    None,
                    format!("duplicate tag '{}'", tag),
                ));
            }
        }

        // Recommendation without visit
        if let Some(note) = &obj.author_note
            && note.recommendation.is_some()
            && note.visit_status != VisitStatus::Visited
        {
            diagnostics.push(diagnostic(
                "recommendation_without_visit",
                &obj.source_path,
                Some(obj.key.clone()),
                None,
                "recommendation requires visit_status to be 'visited'",
            ));
        }

        // Source URLs
        for source in &obj.sources {
            if !is_valid_http_url(&source.url) {
                diagnostics.push(diagnostic(
                    "invalid_source_url",
                    &obj.source_path,
                    Some(obj.key.clone()),
                    None,
                    format!("invalid source URL: '{}'", source.url),
                ));
            }
        }

        // Photo paths: check path traversal / escape
        for photo in &obj.photos {
            if has_path_escape(&photo.path) {
                diagnostics.push(diagnostic(
                    "path_traversal",
                    &obj.source_path,
                    Some(obj.key.clone()),
                    None,
                    format!("photo path escapes permitted directory: '{}'", photo.path),
                ));
            }
        }

        // Kind-specific checks
        match &obj.payload {
            CatalogPayload::Place { coordinates, .. } => {
                validate_coordinate(coordinates, &obj.source_path, &obj.key, &mut diagnostics);
            }
            CatalogPayload::Route {
                route_type,
                geometry_path,
                stops,
                ..
            } => {
                if has_path_escape(geometry_path) {
                    diagnostics.push(diagnostic(
                        "path_traversal",
                        &obj.source_path,
                        Some(obj.key.clone()),
                        None,
                        format!(
                            "geometry path escapes permitted directory: '{}'",
                            geometry_path
                        ),
                    ));
                }

                // Route stops validation (checked in second pass against known_keys)
                for stop in stops {
                    let place_key = ObjectKey {
                        kind: ObjectKind::Place,
                        id: stop.place_id.clone(),
                    };
                    if !object_map.contains_key(&place_key) {
                        // Will check in second pass or defer
                    }
                }

                // Geometry checks
                if let Some(coords) = &obj.geometry {
                    if coords.len() < 2 {
                        diagnostics.push(diagnostic(
                            "invalid_geometry",
                            &obj.source_path,
                            Some(obj.key.clone()),
                            None,
                            "route geometry must contain at least 2 coordinates",
                        ));
                    } else {
                        for coord in coords {
                            validate_coordinate(
                                coord,
                                &obj.source_path,
                                &obj.key,
                                &mut diagnostics,
                            );
                        }

                        // Loop closure check
                        if *route_type == RouteType::Loop {
                            let first = coords[0];
                            let last = coords[coords.len() - 1];
                            if first != last {
                                diagnostics.push(diagnostic(
                                    "loop_not_closed",
                                    &obj.source_path,
                                    Some(obj.key.clone()),
                                    Some([0, coords.len() - 1]),
                                    format!(
                                        "loop route first coordinate {:?} does not match last coordinate {:?}",
                                        first, last
                                    ),
                                ));
                            }
                        }

                        // Spur detection
                        if audit_spurs {
                            let spurs = detect_spurs(coords);
                            for spur in spurs {
                                diagnostics.push(diagnostic(
                                    "spur_detected",
                                    &obj.source_path,
                                    Some(obj.key.clone()),
                                    Some([spur.start_index, spur.end_index]),
                                    format!(
                                        "detected spur from index {} to {} (length {:.1}m, gap {:.1}m)",
                                        spur.start_index, spur.end_index, spur.length_m, spur.gap_m
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            CatalogPayload::Road {
                geometry_path,
                surface_evidence,
                ..
            } => {
                if has_path_escape(geometry_path) {
                    diagnostics.push(diagnostic(
                        "path_traversal",
                        &obj.source_path,
                        Some(obj.key.clone()),
                        None,
                        format!(
                            "geometry path escapes permitted directory: '{}'",
                            geometry_path
                        ),
                    ));
                }
                for source in surface_evidence {
                    if !is_valid_http_url(&source.url) {
                        diagnostics.push(diagnostic(
                            "invalid_source_url",
                            &obj.source_path,
                            Some(obj.key.clone()),
                            None,
                            format!("invalid surface evidence URL: '{}'", source.url),
                        ));
                    }
                }
            }
        }
    }

    // Second pass: Cross-object relational checks
    for obj in &catalog.objects {
        // Related objects
        for rel in &obj.related {
            if !known_keys.contains(rel) {
                diagnostics.push(diagnostic(
                    "broken_relation",
                    &obj.source_path,
                    Some(obj.key.clone()),
                    None,
                    format!("related object {:?}:{} does not exist", rel.kind, rel.id),
                ));
            }
        }

        // Stops targets
        if let CatalogPayload::Route { stops, .. } = &obj.payload {
            for stop in stops {
                let place_key = ObjectKey {
                    kind: ObjectKind::Place,
                    id: stop.place_id.clone(),
                };
                if !known_keys.contains(&place_key) {
                    diagnostics.push(diagnostic(
                        "invalid_stop_target",
                        &obj.source_path,
                        Some(obj.key.clone()),
                        None,
                        format!("stop place_id '{}' not found in places", stop.place_id),
                    ));
                }
            }
        }
    }

    // Sort diagnostics deterministically
    diagnostics.sort_by(|a, b| {
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

    diagnostics
}

fn validate_coordinate(
    coord: &Coordinate,
    file: &str,
    key: &ObjectKey,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let [lon, lat] = *coord;
    if !lon.is_finite()
        || !lat.is_finite()
        || !(-180.0..=180.0).contains(&lon)
        || !(-90.0..=90.0).contains(&lat)
    {
        diagnostics.push(diagnostic(
            "invalid_coordinate",
            file,
            Some(key.clone()),
            None,
            format!("coordinate [{}, {}] is invalid or out of range", lon, lat),
        ));
    }
}
