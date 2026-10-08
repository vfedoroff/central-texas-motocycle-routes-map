use crate::graph::{
    RestrictionMember, RoadEdge, RoadGraph, RoadNode, TurnRestriction, haversine_distance_m,
};
use catalog_model::{Bounds, Coordinate, Direction, EdgeId};
use osmpbfreader::{OsmObj, OsmPbfReader};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    path::Path,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("OSM PBF error: {0}")]
    Pbf(#[from] osmpbfreader::Error),
    #[error("Referenced node {0} missing in PBF extract")]
    MissingReferencedNode(i64),
    #[error("No valid road network edges found within bounds")]
    EmptyNetwork,
}

pub fn is_allowed_highway(highway: &str) -> bool {
    matches!(
        highway,
        "motorway"
            | "trunk"
            | "primary"
            | "secondary"
            | "tertiary"
            | "unclassified"
            | "residential"
            | "motorway_link"
            | "trunk_link"
            | "primary_link"
            | "secondary_link"
            | "tertiary_link"
    )
}

pub fn is_oneway(tags: &BTreeMap<String, String>) -> (bool, bool) {
    // Returns (forward_allowed, reverse_allowed)
    if let Some(oneway) = tags.get("oneway").map(|s| s.as_str()) {
        match oneway {
            "yes" | "1" | "true" => (true, false),
            "-1" => (false, true),
            "no" | "0" | "false" => (true, true),
            _ => (true, true),
        }
    } else if let Some(junction) = tags.get("junction").map(|s| s.as_str()) {
        if junction == "roundabout" || junction == "circular" {
            (true, false)
        } else {
            (true, true)
        }
    } else {
        (true, true)
    }
}

pub fn import_network(pbf_path: &Path, bounds: Bounds) -> Result<RoadGraph, ImportError> {
    let [min_lon, min_lat, max_lon, max_lat] = bounds;

    // Expand bounds slightly by 0.05 deg (~5km) for connectivity buffer
    let buf_min_lon = min_lon - 0.05;
    let buf_min_lat = min_lat - 0.05;
    let buf_max_lon = max_lon + 0.05;
    let buf_max_lat = max_lat + 0.05;

    let in_bounds = |lon: f64, lat: f64| -> bool {
        lon >= buf_min_lon && lon <= buf_max_lon && lat >= buf_min_lat && lat <= buf_max_lat
    };

    // Pass 1: Find candidate nodes in bounds and candidate ways
    let file = File::open(pbf_path)?;
    let mut reader = OsmPbfReader::new(file);

    let mut nodes_in_bounds = HashSet::new();
    let mut way_candidates = Vec::new();
    let mut restriction_candidates = Vec::new();

    for obj in reader.iter() {
        let obj = obj?;
        match obj {
            OsmObj::Node(n) => {
                let lon = n.lon();
                let lat = n.lat();
                if in_bounds(lon, lat) {
                    nodes_in_bounds.insert(n.id.0);
                }
            }
            OsmObj::Way(w) => {
                if let Some(h) = w.tags.get("highway")
                    && is_allowed_highway(h)
                {
                    way_candidates.push(w);
                }
            }
            OsmObj::Relation(r) => {
                if r.tags.get("type").map(|s| s.as_str()) == Some("restriction") {
                    restriction_candidates.push(r);
                }
            }
        }
    }

    // Filter ways: must contain at least one node inside the bounds buffer
    let mut selected_ways = Vec::new();
    let mut needed_node_ids = HashSet::new();
    let mut selected_way_ids = HashSet::new();

    for w in way_candidates {
        if w.nodes
            .iter()
            .any(|node_id| nodes_in_bounds.contains(&node_id.0))
        {
            selected_way_ids.insert(w.id.0);
            for node_id in &w.nodes {
                needed_node_ids.insert(node_id.0);
            }
            selected_ways.push(w);
        }
    }

    if selected_ways.is_empty() {
        return Err(ImportError::EmptyNetwork);
    }

    // Filter turn restrictions: must reference at least one selected way
    let mut selected_restrictions = Vec::new();
    for r in restriction_candidates {
        let touches_selected = r.refs.iter().any(|m| {
            matches!(m.member, osmpbfreader::OsmId::Way(w_id) if selected_way_ids.contains(&w_id.0))
        });
        if touches_selected {
            let mut tags = BTreeMap::new();
            for (k, v) in r.tags.iter() {
                tags.insert(k.to_string(), v.to_string());
            }
            let members = r
                .refs
                .iter()
                .map(|m| {
                    let (member_type, ref_id) = match m.member {
                        osmpbfreader::OsmId::Node(id) => ("node", id.0),
                        osmpbfreader::OsmId::Way(id) => ("way", id.0),
                        osmpbfreader::OsmId::Relation(id) => ("relation", id.0),
                    };
                    RestrictionMember {
                        role: m.role.to_string(),
                        member_type: member_type.into(),
                        ref_id,
                    }
                })
                .collect();
            selected_restrictions.push(TurnRestriction {
                relation_id: r.id.0,
                tags,
                members,
            });
        }
    }

    // Pass 2: Re-read to collect coordinates of all needed nodes
    let file2 = File::open(pbf_path)?;
    let mut reader2 = OsmPbfReader::new(file2);
    let mut node_coords: HashMap<i64, Coordinate> = HashMap::new();

    for obj in reader2.iter() {
        let obj = obj?;
        if let OsmObj::Node(n) = obj
            && needed_node_ids.contains(&n.id.0)
        {
            node_coords.insert(n.id.0, [n.lon(), n.lat()]);
        }
    }

    // Build RoadNodes
    let mut nodes = Vec::with_capacity(node_coords.len());
    for (id, coords) in &node_coords {
        nodes.push(RoadNode {
            id: *id,
            coordinates: *coords,
        });
    }

    // Build RoadEdges
    let mut edges = Vec::new();
    for w in selected_ways {
        let mut tags = BTreeMap::new();
        for (k, v) in w.tags.iter() {
            tags.insert(k.to_string(), v.to_string());
        }
        let (fwd_allowed, rev_allowed) = is_oneway(&tags);

        for (idx, win) in w.nodes.windows(2).enumerate() {
            let n1_id = win[0].0;
            let n2_id = win[1].0;

            let c1 = node_coords
                .get(&n1_id)
                .ok_or(ImportError::MissingReferencedNode(n1_id))?;
            let c2 = node_coords
                .get(&n2_id)
                .ok_or(ImportError::MissingReferencedNode(n2_id))?;
            let length_m = haversine_distance_m(*c1, *c2);

            let segment_index = idx as u32;

            if fwd_allowed {
                edges.push(RoadEdge {
                    id: EdgeId {
                        way_id: w.id.0,
                        segment_index,
                        direction: Direction::Forward,
                    },
                    from: n1_id,
                    to: n2_id,
                    tags: tags.clone(),
                    length_m,
                });
            }

            if rev_allowed {
                edges.push(RoadEdge {
                    id: EdgeId {
                        way_id: w.id.0,
                        segment_index,
                        direction: Direction::Reverse,
                    },
                    from: n2_id,
                    to: n1_id,
                    tags: tags.clone(),
                    length_m,
                });
            }
        }
    }

    Ok(RoadGraph::new(nodes, edges, selected_restrictions))
}
