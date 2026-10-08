use crate::graph::RoadEdge;
use catalog_model::EdgeId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewField {
    Surface,
    Access,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub edge_ids: Vec<EdgeId>,
    pub field: ReviewField,
    pub asserted_value: String,
    pub source_url: String,
    pub checked_on: String,
    pub reviewer: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    ReviewRequired(String),
    Ineligible(String),
}

pub fn is_paved_surface(surface: &str) -> Option<bool> {
    match surface {
        "asphalt" | "concrete" | "concrete:lanes" | "concrete:plates" | "paved" => Some(true),
        "gravel" | "dirt" | "ground" | "sand" | "unpaved" | "fine_gravel" | "earth" | "mud" => {
            Some(false)
        }
        "paving_stones" | "cobblestone" | "sett" | "compacted" => None, // requires review
        _ => None,
    }
}

pub fn classify_edge(edge: &RoadEdge, reviews: &[Review]) -> Eligibility {
    // 1. Check road class / highway tag
    if let Some(highway) = edge.tags.get("highway") {
        match highway.as_str() {
            "motorway" | "trunk" | "primary" | "secondary" | "tertiary" | "unclassified"
            | "residential" | "motorway_link" | "trunk_link" | "primary_link"
            | "secondary_link" | "tertiary_link" => {}
            "service" | "track" | "path" | "footway" | "cycleway" | "construction" | "proposed"
            | "bridleway" | "steps" => {
                return Eligibility::Ineligible(format!("road class '{}' not permitted", highway));
            }
            other => {
                return Eligibility::ReviewRequired(format!(
                    "unrecognized highway class '{}'",
                    other
                ));
            }
        }
    } else {
        return Eligibility::ReviewRequired("missing highway tag".into());
    }

    // 2. Check access policy (precedence: motorcycle > motor_vehicle > vehicle > access)
    let access_val = edge
        .tags
        .get("motorcycle")
        .or_else(|| edge.tags.get("motor_vehicle"))
        .or_else(|| edge.tags.get("vehicle"))
        .or_else(|| edge.tags.get("access"))
        .map(|s| s.as_str());

    // Explicit denial and unpaved facts cannot be overridden by ordinary reviews.
    if matches!(access_val, Some("no" | "private")) {
        return Eligibility::Ineligible("access prohibited by reference data".into());
    }
    if edge.tags.get("surface").and_then(|s| is_paved_surface(s)) == Some(false) {
        return Eligibility::Ineligible("unpaved surface in reference data".into());
    }
    if edge.tags.keys().any(|k| k.ends_with(":conditional")) {
        return Eligibility::ReviewRequired("unsupported conditional road policy".into());
    }
    if !matches!(access_val, Some("yes" | "permissive")) {
        let reviewed = reviews.iter().any(|r| {
            valid_review(r)
                && r.field == ReviewField::Access
                && r.edge_ids.contains(&edge.id)
                && matches!(r.asserted_value.as_str(), "yes" | "permissive")
        });
        if !reviewed {
            return Eligibility::ReviewRequired(
                "missing or ambiguous access requires review".into(),
            );
        }
    }

    // 3. Check surface policy
    let surface_tag = edge.tags.get("surface").map(|s| s.as_str());
    match surface_tag {
        Some(surf) => match is_paved_surface(surf) {
            Some(true) => Eligibility::Eligible,
            Some(false) => {
                // Explicit unpaved
                Eligibility::Ineligible(format!("unpaved surface '{}'", surf))
            }
            None => {
                // Ambiguous/stone surface: check reviews
                let reviewed = reviews.iter().any(|r| {
                    valid_review(r)
                        && r.field == ReviewField::Surface
                        && r.edge_ids.contains(&edge.id)
                        && r.asserted_value == "paved"
                });
                if reviewed {
                    Eligibility::Eligible
                } else {
                    Eligibility::ReviewRequired(format!("surface '{}' requires review", surf))
                }
            }
        },
        None => {
            // Missing surface tag: major roads (motorway, trunk, primary) are conventionally paved,
            // but secondary/tertiary/unclassified/residential without surface require review per spec
            let reviewed = reviews.iter().any(|r| {
                valid_review(r)
                    && r.field == ReviewField::Surface
                    && r.edge_ids.contains(&edge.id)
                    && r.asserted_value == "paved"
            });
            if reviewed {
                Eligibility::Eligible
            } else {
                Eligibility::ReviewRequired("missing surface tag requires manual review".into())
            }
        }
    }
}

pub fn valid_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || b.iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return false;
    }
    let year = value[..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..].parse::<u32>().unwrap_or(0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 0,
    };
    year > 0 && day > 0 && day <= days
}
pub fn valid_source_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && value
            .split("://")
            .nth(1)
            .is_some_and(|s| !s.is_empty() && !s.starts_with('/'))
        && !value.chars().any(char::is_whitespace)
}
pub fn valid_review(r: &Review) -> bool {
    !r.edge_ids.is_empty()
        && valid_source_url(&r.source_url)
        && valid_date(&r.checked_on)
        && !r.reviewer.trim().is_empty()
        && !r.reason.trim().is_empty()
}
