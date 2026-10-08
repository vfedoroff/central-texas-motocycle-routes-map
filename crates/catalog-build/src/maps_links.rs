use catalog_model::{CatalogObject, CatalogPayload, NavigationLink, NavigationMode};
use url::Url;

const MAX_URL_LENGTH: usize = 2048;

/// Build Google Maps navigation links for a catalog object.
/// - Places: destination directions with optional Google Place ID (mode: Destination).
/// - Roads / Routes without verified junctions: directions to first coordinate (mode: Start).
/// - Routes with verified junctions: staged navigation URLs up to 5 points per stage (mode: Stage).
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
            let junction_points: Vec<[f64; 2]> = navigation_junctions
                .iter()
                .filter_map(|j| j.coordinates)
                .collect();

            if junction_points.len() >= 2 {
                build_stage_links(&object.title, &junction_points)
            } else {
                directions_to_start(object)
            }
        }
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
