use std::collections::HashSet;

use catalog_model::{Bounds, CatalogIndex, CatalogSummary, ObjectKey, ObjectKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MileageFilterError {
    #[error("Mileage cannot be negative")]
    Negative,
    #[error("Mileage must be a valid number")]
    NonFinite,
    #[error("Minimum distance cannot exceed maximum distance")]
    Inverted,
}

pub fn validate_mileage(min: Option<f64>, max: Option<f64>) -> Result<(), MileageFilterError> {
    if let Some(m) = min {
        if !m.is_finite() {
            return Err(MileageFilterError::NonFinite);
        }
        if m < 0.0 {
            return Err(MileageFilterError::Negative);
        }
    }
    if let Some(m) = max {
        if !m.is_finite() {
            return Err(MileageFilterError::NonFinite);
        }
        if m < 0.0 {
            return Err(MileageFilterError::Negative);
        }
    }
    if let (Some(min_val), Some(max_val)) = (min, max)
        && min_val > max_val
    {
        return Err(MileageFilterError::Inverted);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSort {
    #[default]
    Title,
    DistanceAscending,
    DistanceDescending,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogFilter {
    pub kind: ObjectKind,
    pub query: String,
    pub categories: Vec<String>,
    pub min_distance_mi: Option<f64>,
    pub max_distance_mi: Option<f64>,
    pub applied_bounds: Option<Bounds>,
    pub saved_only: bool,
}

impl Default for CatalogFilter {
    fn default() -> Self {
        Self {
            kind: ObjectKind::Route,
            query: String::new(),
            categories: Vec::new(),
            min_distance_mi: None,
            max_distance_mi: None,
            applied_bounds: None,
            saved_only: false,
        }
    }
}

pub fn bounds_intersect(a: &Bounds, b: &Bounds) -> bool {
    // a = [w1, s1, e1, n1], b = [w2, s2, e2, n2]
    a[0] <= b[2] && a[2] >= b[0] && a[1] <= b[3] && a[3] >= b[1]
}

pub fn haversine_distance_mi(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    const EARTH_RADIUS_MI: f64 = 3958.8;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a =
        (d_lat / 2.0).sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    EARTH_RADIUS_MI * c
}

pub fn distance_to_user_mi(summary: &CatalogSummary, user_loc: &[f64; 2]) -> f64 {
    let [user_lon, user_lat] = *user_loc;
    if let Some([p_lon, p_lat]) = summary.point {
        haversine_distance_mi(user_lon, user_lat, p_lon, p_lat)
    } else {
        let center_lon = (summary.bounds[0] + summary.bounds[2]) / 2.0;
        let center_lat = (summary.bounds[1] + summary.bounds[3]) / 2.0;
        haversine_distance_mi(user_lon, user_lat, center_lon, center_lat)
    }
}

pub fn filter_summaries(index: &CatalogIndex, filter: &CatalogFilter) -> Vec<ObjectKey> {
    filter_summaries_with_saved(index, filter, None)
}

pub fn filter_summaries_with_saved(
    index: &CatalogIndex,
    filter: &CatalogFilter,
    saved_keys: Option<&HashSet<ObjectKey>>,
) -> Vec<ObjectKey> {
    filter_summaries_with_location(index, filter, saved_keys, None)
}

pub fn filter_summaries_with_location(
    index: &CatalogIndex,
    filter: &CatalogFilter,
    saved_keys: Option<&HashSet<ObjectKey>>,
    user_location: Option<&[f64; 2]>,
) -> Vec<ObjectKey> {
    let tokens: Vec<String> = filter
        .query
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .collect();

    let mut matched: Vec<&CatalogSummary> = index
        .objects
        .iter()
        .filter(|summary| {
            // 1. Kind filter
            if summary.key.kind != filter.kind {
                return false;
            }

            // 2. Saved-only filter
            if filter.saved_only {
                match saved_keys {
                    Some(keys) if keys.contains(&summary.key) => {}
                    _ => return false,
                }
            }

            // 3. Query tokens (AND)
            if !tokens.is_empty() {
                let search_text = summary.search_text.to_lowercase();
                for token in &tokens {
                    if !search_text.contains(token) {
                        return false;
                    }
                }
            }

            // 4. Bounds intersection
            if let Some(ref bounds) = filter.applied_bounds
                && !bounds_intersect(&summary.bounds, bounds)
            {
                return false;
            }

            // 5. Category and Mileage (apply to routes only)
            if summary.key.kind == ObjectKind::Route {
                if !filter.categories.is_empty() {
                    match &summary.category {
                        Some(cat) => {
                            if !filter
                                .categories
                                .iter()
                                .any(|c| c.eq_ignore_ascii_case(cat))
                            {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }

                if let Some(min_dist) = filter.min_distance_mi {
                    match summary.distance_mi {
                        Some(dist) => {
                            if dist < min_dist {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }

                if let Some(max_dist) = filter.max_distance_mi {
                    match summary.distance_mi {
                        Some(dist) => {
                            if dist > max_dist {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
            }

            true
        })
        .collect();

    if let Some(user_loc) = user_location {
        matched.sort_by(|a, b| {
            let dist_a = distance_to_user_mi(a, user_loc);
            let dist_b = distance_to_user_mi(b, user_loc);
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let title_cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
                    if title_cmp != std::cmp::Ordering::Equal {
                        title_cmp
                    } else {
                        (a.key.kind, &a.key.id).cmp(&(b.key.kind, &b.key.id))
                    }
                })
        });
    } else {
        // Sort by lowercase title, then kind, then ID
        matched.sort_by(|a, b| {
            let title_cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
            if title_cmp != std::cmp::Ordering::Equal {
                return title_cmp;
            }
            (a.key.kind, &a.key.id).cmp(&(b.key.kind, &b.key.id))
        });
    }

    matched.into_iter().map(|s| s.key.clone()).collect()
}

pub fn sort_result_keys(
    index: &CatalogIndex,
    keys: Vec<ObjectKey>,
    sort: CatalogSort,
) -> Vec<ObjectKey> {
    if keys.is_empty() {
        return keys;
    }
    use std::collections::HashMap;
    let summary_map: HashMap<&ObjectKey, &CatalogSummary> =
        index.objects.iter().map(|s| (&s.key, s)).collect();

    let mut sorted = keys;
    sorted.sort_by(|key_a, key_b| {
        let sum_a = summary_map.get(key_a);
        let sum_b = summary_map.get(key_b);

        let title_cmp = || {
            let t_a = sum_a.map(|s| s.title.to_lowercase()).unwrap_or_default();
            let t_b = sum_b.map(|s| s.title.to_lowercase()).unwrap_or_default();
            let c = t_a.cmp(&t_b);
            if c != std::cmp::Ordering::Equal {
                c
            } else {
                (key_a.kind, &key_a.id).cmp(&(key_b.kind, &key_b.id))
            }
        };

        match sort {
            CatalogSort::Title => title_cmp(),
            CatalogSort::DistanceAscending => {
                let dist_a = sum_a.and_then(|s| s.distance_mi);
                let dist_b = sum_b.and_then(|s| s.distance_mi);
                match (dist_a, dist_b) {
                    (Some(da), Some(db)) => {
                        let c = da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal);
                        if c != std::cmp::Ordering::Equal {
                            c
                        } else {
                            title_cmp()
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => title_cmp(),
                }
            }
            CatalogSort::DistanceDescending => {
                let dist_a = sum_a.and_then(|s| s.distance_mi);
                let dist_b = sum_b.and_then(|s| s.distance_mi);
                match (dist_a, dist_b) {
                    (Some(da), Some(db)) => {
                        let c = db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal);
                        if c != std::cmp::Ordering::Equal {
                            c
                        } else {
                            title_cmp()
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => title_cmp(),
                }
            }
        }
    });

    sorted
}

pub fn filter_places_overlay(
    index: &CatalogIndex,
    query: &str,
    applied_bounds: Option<&Bounds>,
) -> Vec<ObjectKey> {
    let tokens: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();

    let mut matched: Vec<&CatalogSummary> = index
        .objects
        .iter()
        .filter(|summary| {
            if summary.key.kind != ObjectKind::Place {
                return false;
            }

            if !tokens.is_empty() {
                let search_text = summary.search_text.to_lowercase();
                for token in &tokens {
                    if !search_text.contains(token) {
                        return false;
                    }
                }
            }

            if let Some(bounds) = applied_bounds
                && !bounds_intersect(&summary.bounds, bounds)
            {
                return false;
            }

            true
        })
        .collect();

    matched.sort_by(|a, b| {
        let title_cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
        if title_cmp != std::cmp::Ordering::Equal {
            return title_cmp;
        }
        (a.key.kind, &a.key.id).cmp(&(b.key.kind, &b.key.id))
    });

    matched.into_iter().map(|s| s.key.clone()).collect()
}

pub fn filter_summaries_for_overview<'a>(
    index: &'a CatalogIndex,
    filter: &CatalogFilter,
    vp_bounds: &Bounds,
) -> Vec<&'a CatalogSummary> {
    if filter.kind == ObjectKind::Place {
        return Vec::new();
    }

    let tokens: Vec<String> = filter
        .query
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .collect();

    let mut matched: Vec<&CatalogSummary> = index
        .objects
        .iter()
        .filter(|summary| {
            if summary.overview_url.is_none() {
                return false;
            }

            if summary.key.kind != filter.kind {
                return false;
            }

            if !tokens.is_empty() {
                let st = summary.search_text.to_lowercase();
                for token in &tokens {
                    if !st.contains(token) {
                        return false;
                    }
                }
            }

            if summary.key.kind == ObjectKind::Route && !filter.categories.is_empty() {
                if let Some(ref cat) = summary.category {
                    if !filter.categories.contains(cat) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            if summary.key.kind == ObjectKind::Route
                && let Some(dist) = summary.distance_mi
            {
                if let Some(min) = filter.min_distance_mi
                    && dist < min
                {
                    return false;
                }
                if let Some(max) = filter.max_distance_mi
                    && dist > max
                {
                    return false;
                }
            }

            if !bounds_intersect(&summary.bounds, vp_bounds) {
                return false;
            }

            true
        })
        .collect();

    matched.sort_by(|a, b| {
        let title_cmp = a.title.to_lowercase().cmp(&b.title.to_lowercase());
        if title_cmp != std::cmp::Ordering::Equal {
            return title_cmp;
        }
        (a.key.kind, &a.key.id).cmp(&(b.key.kind, &b.key.id))
    });

    matched
}
