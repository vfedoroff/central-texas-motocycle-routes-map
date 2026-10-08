use catalog_model::{
    Bounds, Catalog, CatalogIndex, CatalogObject, CatalogPayload, CatalogSummary, Coordinate,
    ObjectKind,
};

pub fn compute_bounds(coords: &[Coordinate]) -> Bounds {
    if coords.is_empty() {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let mut min_lon = coords[0][0];
    let mut min_lat = coords[0][1];
    let mut max_lon = coords[0][0];
    let mut max_lat = coords[0][1];

    for p in &coords[1..] {
        if p[0] < min_lon {
            min_lon = p[0];
        }
        if p[0] > max_lon {
            max_lon = p[0];
        }
        if p[1] < min_lat {
            min_lat = p[1];
        }
        if p[1] > max_lat {
            max_lat = p[1];
        }
    }

    [min_lon, min_lat, max_lon, max_lat]
}

pub fn build_index(catalog: &Catalog) -> CatalogIndex {
    let mut summaries = Vec::with_capacity(catalog.objects.len());

    for obj in &catalog.objects {
        let summary = build_summary(obj);
        summaries.push(summary);
    }

    // Sort deterministically by kind, then ID
    summaries.sort_by(|a, b| {
        a.key
            .kind
            .cmp(&b.key.kind)
            .then_with(|| a.key.id.cmp(&b.key.id))
    });

    CatalogIndex {
        schema_version: 1,
        objects: summaries,
    }
}

pub fn build_summary(obj: &CatalogObject) -> CatalogSummary {
    let kind_str = match obj.key.kind {
        ObjectKind::Route => "routes",
        ObjectKind::Road => "roads",
        ObjectKind::Place => "places",
    };

    let page_url = format!("/{}/{}/index.html", kind_str, obj.key.id);
    let detail_url = format!("/data/{}/{}.json", kind_str, obj.key.id);

    let (
        bounds,
        point,
        overview_url,
        geometry_url,
        category,
        color,
        distance_mi,
        place_cat,
        route_type,
        start_label,
        extra_search,
    ) = match &obj.payload {
        CatalogPayload::Route {
            category,
            color,
            distance_mi,
            waypoints,
            via,
            route_type,
            navigation_junctions,
            ..
        } => {
            let coords = obj.geometry.as_deref().unwrap_or(&[]);
            let b = compute_bounds(coords);
            let mut search_parts = vec![via.clone()];
            search_parts.extend(waypoints.iter().cloned());
            let start = navigation_junctions
                .first()
                .filter(|j| j.coordinates.is_some() && !j.label.trim().is_empty())
                .map(|j| j.label.clone());
            (
                b,
                None,
                Some(format!("/data/overview/routes/{}.geojson", obj.key.id)),
                Some(format!("/data/geometry/routes/{}.geojson", obj.key.id)),
                Some(category.clone()),
                Some(color.clone()),
                Some(*distance_mi),
                None,
                Some(*route_type),
                start,
                search_parts.join(" "),
            )
        }
        CatalogPayload::Road { .. } => {
            let coords = obj.geometry.as_deref().unwrap_or(&[]);
            let b = compute_bounds(coords);
            (
                b,
                None,
                Some(format!("/data/overview/roads/{}.geojson", obj.key.id)),
                Some(format!("/data/geometry/roads/{}.geojson", obj.key.id)),
                None,
                None,
                None,
                None,
                None,
                None,
                String::new(),
            )
        }
        CatalogPayload::Place {
            place_category,
            coordinates,
            ..
        } => {
            let b = [
                coordinates[0],
                coordinates[1],
                coordinates[0],
                coordinates[1],
            ];
            (
                b,
                Some(*coordinates),
                None,
                None,
                None,
                None,
                None,
                Some(*place_category),
                None,
                None,
                String::new(),
            )
        }
    };

    let mut search_tokens = Vec::new();
    search_tokens.push(obj.title.to_lowercase());
    search_tokens.push(obj.summary.to_lowercase());
    for t in &obj.tags {
        search_tokens.push(t.to_lowercase());
    }
    if !extra_search.is_empty() {
        search_tokens.push(extra_search.to_lowercase());
    }
    let search_text = search_tokens
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    CatalogSummary {
        key: obj.key.clone(),
        title: obj.title.clone(),
        summary: obj.summary.clone(),
        tags: obj.tags.clone(),
        search_text,
        bounds,
        point,
        category,
        color,
        distance_mi,
        place_category: place_cat,
        route_type,
        start_label,
        page_url,
        detail_url,
        overview_url,
        geometry_url,
    }
}
