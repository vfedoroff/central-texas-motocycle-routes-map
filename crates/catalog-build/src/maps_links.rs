use catalog_model::{CatalogObject, CatalogPayload, NavigationLink, NavigationMode};
use url::Url;

const MAX_URL_LENGTH: usize = 2048;

/// Build Google Maps navigation links for a catalog object.
/// - Places: destination directions with optional Google Place ID (mode: Destination).
/// - Roads: directions to first coordinate (mode: Start).
/// - Routes: authored Google Maps itinerary, verified junctions, or simplified geometry.
/// - Route links use at most five points when generated (mode: Stage).
pub fn google_maps_links(object: &CatalogObject) -> Vec<NavigationLink> {
    match &object.payload {
        CatalogPayload::Place {
            coordinates,
            google_place_id,
            ..
        } => {
            let mut url = Url::parse("https://www.google.com/maps/dir/").expect("valid base URL");
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("api", "1");
                query.append_pair(
                    "destination",
                    &format!("{},{}", coordinates[1], coordinates[0]),
                );
                query.append_pair("travelmode", "driving");
                if let Some(place_id) = google_place_id {
                    query.append_pair("destination_place_id", place_id);
                }
            }
            let url_str = url.to_string();
            if url_str.len() > MAX_URL_LENGTH {
                eprintln!(
                    "[Warning] MAPS_LINK_OVERFLOW: Place link exceeds 2,048 chars for '{}'",
                    object.key.id
                );
                Vec::new()
            } else {
                vec![NavigationLink {
                    label: format!("Directions to {}", object.title),
                    url: url_str,
                    mode: NavigationMode::Destination,
                }]
            }
        }
        CatalogPayload::Road { .. } => directions_to_start(object),
        CatalogPayload::Route {
            navigation_junctions,
            ..
        } => {
            if let Some(source) = object.sources.iter().find(|source| {
                Url::parse(&source.url).is_ok_and(|url| {
                    url.scheme() == "https"
                        && matches!(url.host_str(), Some("www.google.com" | "google.com"))
                        && url.path().starts_with("/maps/dir/")
                        && source.url.len() <= MAX_URL_LENGTH
                })
            }) {
                return vec![NavigationLink {
                    label: "Open ride in Google Maps".to_string(),
                    url: source.url.clone(),
                    mode: NavigationMode::Stage,
                }];
            }
            let junction_points: Vec<[f64; 2]> = navigation_junctions
                .iter()
                .filter_map(|j| j.coordinates)
                .collect();

            if junction_points.len() >= 2 {
                build_route_link(&object.title, &junction_points)
            } else {
                let Some(geometry) = &object.geometry else {
                    return Vec::new();
                };
                let points = simplify_navigation_points(geometry);
                build_route_link(&object.title, &points)
            }
        }
    }
}

/// Keep one link with at most three intermediate points for mobile Maps URLs.
fn build_route_link(title: &str, points: &[[f64; 2]]) -> Vec<NavigationLink> {
    let selected = if points.len() > 5 {
        (0..5)
            .map(|i| points[i * (points.len() - 1) / 4])
            .collect::<Vec<_>>()
    } else {
        points.to_vec()
    };
    let mut links = build_stage_links(title, &selected);
    if let Some(link) = links.first_mut() {
        link.label = "Open ride in Google Maps".to_string();
    }
    links
}

/// Preserve bends and loop endpoints while reducing dense road geometry.
/// Google Maps recalculates roads between these navigation points.
fn simplify_navigation_points(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let first = points[0];
    let last = points[points.len() - 1];
    let dx = last[0] - first[0];
    let dy = last[1] - first[1];
    let length_sq = dx * dx + dy * dy;
    let mut farthest = (0, 0.0_f64);
    for (index, point) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let t = if length_sq == 0.0 {
            0.0
        } else {
            (((point[0] - first[0]) * dx + (point[1] - first[1]) * dy) / length_sq).clamp(0.0, 1.0)
        };
        let distance =
            (point[0] - first[0] - t * dx).powi(2) + (point[1] - first[1] - t * dy).powi(2);
        if distance > farthest.1 {
            farthest = (index, distance);
        }
    }
    if farthest.1 > 0.005_f64.powi(2) {
        let mut left = simplify_navigation_points(&points[..=farthest.0]);
        left.pop();
        left.extend(simplify_navigation_points(&points[farthest.0..]));
        left
    } else {
        vec![first, last]
    }
}

/// Fallback for roads and routes: directions to first source coordinate.
fn directions_to_start(object: &CatalogObject) -> Vec<NavigationLink> {
    if let Some(coords) = &object.geometry
        && let Some(first) = coords.first()
    {
        let mut url = Url::parse("https://www.google.com/maps/dir/").expect("valid base URL");
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("api", "1");
            query.append_pair("destination", &format!("{},{}", first[1], first[0]));
            query.append_pair("travelmode", "driving");
        }
        let url_str = url.to_string();
        if url_str.len() > MAX_URL_LENGTH {
            eprintln!(
                "[Warning] MAPS_LINK_OVERFLOW: Start link exceeds 2,048 chars for '{}'",
                object.key.id
            );
            Vec::new()
        } else {
            vec![NavigationLink {
                label: format!("Directions to start of {}", object.title),
                url: url_str,
                mode: NavigationMode::Start,
            }]
        }
    } else {
        Vec::new()
    }
}

/// Build staged navigation links from ordered junction points.
/// Maximum five total points per stage (origin + 3 waypoints + destination).
/// Next stage starts at the preceding endpoint.
pub fn build_stage_links(title: &str, points: &[[f64; 2]]) -> Vec<NavigationLink> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }

    let mut stages = Vec::new();
    let mut start_idx = 0;
    while start_idx + 1 < n {
        let end_idx = (start_idx + 4).min(n - 1);
        stages.push(&points[start_idx..=end_idx]);
        start_idx = end_idx;
    }

    let num_stages = stages.len();
    let mut links = Vec::new();

    for (i, stage_pts) in stages.into_iter().enumerate() {
        let origin = stage_pts[0];
        let destination = stage_pts[stage_pts.len() - 1];
        let waypoints = &stage_pts[1..stage_pts.len() - 1];

        let mut url = Url::parse("https://www.google.com/maps/dir/").expect("valid base URL");
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("api", "1");
            query.append_pair("origin", &format!("{},{}", origin[1], origin[0]));
            query.append_pair(
                "destination",
                &format!("{},{}", destination[1], destination[0]),
            );
            if !waypoints.is_empty() {
                let wp_str = waypoints
                    .iter()
                    .map(|p| format!("{},{}", p[1], p[0]))
                    .collect::<Vec<_>>()
                    .join("|");
                query.append_pair("waypoints", &wp_str);
            }
            query.append_pair("travelmode", "driving");
        }

        let url_str = url.to_string();
        if url_str.len() > MAX_URL_LENGTH {
            eprintln!(
                "[Warning] MAPS_LINK_OVERFLOW: Stage {} link exceeds 2,048 chars for '{}', omitted",
                i + 1,
                title
            );
            continue;
        }

        let label = if num_stages == 1 {
            "Stage 1".to_string()
        } else {
            format!("Stage {}", i + 1)
        };

        links.push(NavigationLink {
            label,
            url: url_str,
            mode: NavigationMode::Stage,
        });
    }

    links
}
