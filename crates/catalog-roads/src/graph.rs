use catalog_model::{Coordinate, EdgeId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write, io::Read};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestrictionMember {
    pub role: String,
    #[serde(rename = "type")]
    pub member_type: String,
    #[serde(rename = "ref")]
    pub ref_id: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRestriction {
    pub relation_id: i64,
    pub tags: BTreeMap<String, String>,
    pub members: Vec<RestrictionMember>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadNode {
    pub id: i64,
    pub coordinates: Coordinate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadEdge {
    pub id: EdgeId,
    pub from: i64,
    pub to: i64,
    pub tags: BTreeMap<String, String>,
    pub length_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadGraph {
    pub schema_version: u32,
    pub nodes: Vec<RoadNode>,
    pub edges: Vec<RoadEdge>,
    pub restrictions: Vec<TurnRestriction>,
}

pub fn haversine_distance_m(p1: Coordinate, p2: Coordinate) -> f64 {
    const R: f64 = 6371008.8; // Earth mean radius in meters
    let lat1 = p1[1].to_radians();
    let lat2 = p2[1].to_radians();
    let dlat = (p2[1] - p1[1]).to_radians();
    let dlon = (p2[0] - p1[0]).to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}

impl RoadGraph {
    pub fn new(
        mut nodes: Vec<RoadNode>,
        mut edges: Vec<RoadEdge>,
        mut restrictions: Vec<TurnRestriction>,
    ) -> Self {
        nodes.sort_by_key(|n| n.id);
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        restrictions.sort_by_key(|r| r.relation_id);

        Self {
            schema_version: 1,
            nodes,
            edges,
            restrictions,
        }
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&serde_json::to_value(self)?)
    }

    pub fn canonical_sha256(&self) -> String {
        let bytes = self
            .canonical_bytes()
            .expect("canonical serialization should never fail");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for b in hash {
            let _ = write!(hex, "{:02x}", b);
        }
        hex
    }

    pub fn find_node(&self, id: i64) -> Option<&RoadNode> {
        self.nodes
            .binary_search_by_key(&id, |n| n.id)
            .ok()
            .map(|idx| &self.nodes[idx])
    }

    pub fn find_edge(&self, id: &EdgeId) -> Option<&RoadEdge> {
        self.edges
            .binary_search_by(|e| e.id.cmp(id))
            .ok()
            .map(|idx| &self.edges[idx])
    }

    pub fn edges_from(&self, node_id: i64) -> Vec<&RoadEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    pub fn edges_to(&self, node_id: i64) -> Vec<&RoadEdge> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }

    pub fn encode_zstd(&self) -> Result<Vec<u8>, std::io::Error> {
        let bytes = self
            .canonical_bytes()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        zstd::encode_all(&bytes[..], 19)
    }

    pub fn decode_zstd<R: Read>(reader: R) -> Result<Self, anyhow::Error> {
        let decoded = zstd::decode_all(reader)?;
        let graph: RoadGraph = serde_json::from_slice(&decoded)?;
        Ok(graph)
    }
}
