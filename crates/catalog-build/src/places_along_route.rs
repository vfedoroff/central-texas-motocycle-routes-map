use catalog_model::{
    Catalog, CatalogObject, CatalogPayload, Coordinate, NearbyPlace, ObjectKind, StopDetail,
};
use catalog_roads::haversine_distance_m;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const DEFAULT_NEARBY_RADIUS_M: f64 = 2000.0;
pub const DEFAULT_NEARBY_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct RoutePlaces {
    pub stops: Vec<StopDetail>,
    pub nearby_places: Vec<NearbyPlace>,
}

/// Compute minimum distance in meters from point `p` to a polyline segment `(a, b)`.
pub fn point_to_segment_distance_m(p: Coordinate, a: Coordinate, b: Coordinate) -> f64 {
    let lat_mid = ((a[1] + b[1]) / 2.0).to_radians();
    let cos_lat = lat_mid.cos();

    let dx = (b[0] - a[0]) * cos_lat * 111320.0;
    let dy = (b[1] - a[1]) * 110540.0;
    let seg_len_sq = dx * dx + dy * dy;

    if seg_len_sq <= 1e-6 {
        return haversine_distance_m(p, a);
    }

    let px = (p[0] - a[0]) * cos_lat * 111320.0;
    let py = (p[1] - a[1]) * 110540.0;

    let t = ((px * dx + py * dy) / seg_len_sq).clamp(0.0, 1.0);
    let proj = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];

    haversine_distance_m(p, proj)
}

/// Compute minimum distance in meters from a point to a polyline.
pub fn point_to_polyline_distance_m(p: Coordinate, polyline: &[Coordinate]) -> f64 {
    if polyline.is_empty() {
        return f64::INFINITY;
    }
    if polyline.len() == 1 {
        return haversine_distance_m(p, polyline[0]);
    }

    let mut min_dist = f64::INFINITY;
    for i in 0..polyline.len() - 1 {
        let d = point_to_segment_distance_m(p, polyline[i], polyline[i + 1]);
        if d < min_dist {
            min_dist = d;
        }
    }
    min_dist
}

/// Resolve stops and find nearby places along a route.
pub fn route_places(route: &CatalogObject, catalog: &Catalog) -> RoutePlaces {
    route_places_with_config(
        route,
        catalog,
        DEFAULT_NEARBY_RADIUS_M,
        DEFAULT_NEARBY_LIMIT,
    )
}

/// Resolve stops and find nearby places along a route with custom radius and limit.
pub fn route_places_with_config(
    route: &CatalogObject,
    catalog: &Catalog,
    radius_m: f64,
    limit: usize,
) -> RoutePlaces {
    let authored_stops = match &route.payload {
        CatalogPayload::Route { stops, .. } => stops,
        _ => return RoutePlaces::default(),
    };

    let track_coords = route.geometry.as_deref().unwrap_or(&[]);

    // 1. Build stops in authored order
    let mut resolved_stops = Vec::new();
    let mut stop_place_ids = HashSet::new();

    for stop in authored_stops {
        let place_opt = catalog
            .objects
            .iter()
            .find(|obj| obj.key.kind == ObjectKind::Place && obj.key.id == stop.place_id);

        if let Some(place) = place_opt
            && let CatalogPayload::Place { coordinates, .. } = &place.payload
        {
            stop_place_ids.insert(place.key.id.clone());
            resolved_stops.push(StopDetail {
                key: place.key.clone(),
                title: place.title.clone(),
                page_url: format!("/places/{}/index.html", place.key.id),
                coordinates: *coordinates,
                note: stop.note.clone(),
            });
        }
    }

    // 2. Discover nearby places (excluding explicit stops)
    if track_coords.is_empty() {
        return RoutePlaces {
            stops: resolved_stops,
            nearby_places: Vec::new(),
        };
    }

    let mut nearby_candidates = Vec::new();

    for obj in &catalog.objects {
        if obj.key.kind != ObjectKind::Place {
            continue;
        }

        // Exclude explicit stops
        if stop_place_ids.contains(&obj.key.id) {
            continue;
        }

        if let CatalogPayload::Place { coordinates, .. } = &obj.payload {
            let dist = point_to_polyline_distance_m(*coordinates, track_coords);
            if dist <= radius_m {
                nearby_candidates.push(NearbyPlace {
                    key: obj.key.clone(),
                    title: obj.title.clone(),
                    page_url: format!("/places/{}/index.html", obj.key.id),
                    coordinates: *coordinates,
                    distance_m: (dist * 10.0).round() / 10.0, // 1 decimal place
                });
            }
        }
    }

    // Sort by distance ascending, then stably by title lowercase, then id
    nearby_candidates.sort_by(|a, b| {
        a.distance_m
            .partial_cmp(&b.distance_m)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            .then_with(|| a.key.id.cmp(&b.key.id))
    });

    nearby_candidates.truncate(limit);

    RoutePlaces {
        stops: resolved_stops,
        nearby_places: nearby_candidates,
    }
}
