use serde::{Deserialize, Deserializer, Serialize};

pub type Bounds = [f64; 4];
pub type Coordinate = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Route,
    Road,
    Place,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectKey {
    pub kind: ObjectKind,
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeId {
    pub way_id: i64,
    pub segment_index: u32,
    pub direction: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Review,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub key: Option<ObjectKey>,
    pub file: String,
    pub json_pointer: Option<String>,
    pub coordinate_range: Option<[usize; 2]>,
    pub edge_id: Option<EdgeId>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub title: String,
    pub url: String,
    pub accessed_on: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Photo {
    pub path: String,
    pub alt: String,
    pub credit: String,
    pub rights: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisitStatus {
    Planned,
    Visited,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Recommend,
    Mixed,
    NotRecommended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorNote {
    pub visit_status: VisitStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visited_on: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<Recommendation>,
    pub text: String,
    pub updated_on: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceCategory {
    Viewpoint,
    Food,
    Landmark,
    Stop,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteType {
    Loop,
    Corridor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Paved,
}

impl std::fmt::Display for VisitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Planned => write!(f, "planned"),
            Self::Visited => write!(f, "visited"),
        }
    }
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recommend => write!(f, "recommend"),
            Self::Mixed => write!(f, "mixed"),
            Self::NotRecommended => write!(f, "not_recommended"),
        }
    }
}

impl std::fmt::Display for PlaceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Viewpoint => write!(f, "viewpoint"),
            Self::Food => write!(f, "food"),
            Self::Landmark => write!(f, "landmark"),
            Self::Stop => write!(f, "stop"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl std::fmt::Display for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loop => write!(f, "loop"),
            Self::Corridor => write!(f, "corridor"),
        }
    }
}

impl std::fmt::Display for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paved => write!(f, "paved"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationJunction {
    pub node_id: i64,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredStop {
    pub place_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn nonempty_sources<'de, D>(deserializer: D) -> Result<Vec<Source>, D::Error>
where
    D: Deserializer<'de>,
{
    let sources = Vec::<Source>::deserialize(deserializer)?;
    if sources.is_empty() {
        return Err(serde::de::Error::custom(
            "surface_evidence must be nonempty",
        ));
    }
    Ok(sources)
}

fn nonempty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("value must be nonempty"));
    }
    Ok(value)
}

fn nonempty_strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(serde::de::Error::custom(
            "list must contain nonempty values",
        ));
    }
    Ok(values)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredPlace {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub photos: Vec<Photo>,
    #[serde(default)]
    pub related: Vec<ObjectKey>,
    #[serde(default)]
    pub updated_on: Option<String>,
    #[serde(default)]
    pub author_note: Option<AuthorNote>,
    pub place_category: PlaceCategory,
    pub coordinates: Coordinate,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub google_place_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredRoad {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub photos: Vec<Photo>,
    #[serde(default)]
    pub related: Vec<ObjectKey>,
    #[serde(default)]
    pub updated_on: Option<String>,
    #[serde(default)]
    pub author_note: Option<AuthorNote>,
    pub geometry_path: String,
    pub surface: Surface,
    #[serde(deserialize_with = "nonempty_sources")]
    pub surface_evidence: Vec<Source>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredRoute {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub photos: Vec<Photo>,
    #[serde(default)]
    pub related: Vec<ObjectKey>,
    #[serde(default)]
    pub updated_on: Option<String>,
    #[serde(default)]
    pub author_note: Option<AuthorNote>,
    pub category: String,
    pub color: String,
    pub distance_mi: f64,
    #[serde(deserialize_with = "nonempty_strings")]
    pub waypoints: Vec<String>,
    #[serde(deserialize_with = "nonempty_string")]
    pub via: String,
    pub geometry_path: String,
    pub route_type: RouteType,
    #[serde(default)]
    pub navigation_junctions: Vec<NavigationJunction>,
    #[serde(default)]
    pub stops: Vec<AuthoredStop>,
}

#[cfg(feature = "native")]
#[derive(Clone, Debug)]
pub struct Catalog {
    pub objects: Vec<CatalogObject>,
}

#[cfg(feature = "native")]
#[derive(Clone, Debug)]
pub struct CatalogObject {
    pub schema_version: u32,
    pub source_path: String,
    pub key: ObjectKey,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub tags: Vec<String>,
    pub sources: Vec<Source>,
    pub photos: Vec<Photo>,
    pub related: Vec<ObjectKey>,
    pub updated_on: Option<String>,
    pub author_note: Option<AuthorNote>,
    pub payload: CatalogPayload,
    pub geometry: Option<Vec<Coordinate>>,
}

#[cfg(feature = "native")]
#[derive(Clone, Debug)]
pub enum CatalogPayload {
    Route {
        category: String,
        color: String,
        distance_mi: f64,
        waypoints: Vec<String>,
        via: String,
        geometry_path: String,
        route_type: RouteType,
        navigation_junctions: Vec<NavigationJunction>,
        stops: Vec<AuthoredStop>,
    },
    Road {
        geometry_path: String,
        surface: Surface,
        surface_evidence: Vec<Source>,
    },
    Place {
        place_category: PlaceCategory,
        coordinates: Coordinate,
        address: Option<String>,
        google_place_id: Option<String>,
    },
}

#[cfg(feature = "native")]
impl From<AuthoredPlace> for CatalogObject {
    fn from(value: AuthoredPlace) -> Self {
        Self {
            schema_version: value.schema_version,
            source_path: String::new(),
            key: ObjectKey {
                kind: ObjectKind::Place,
                id: value.id,
            },
            title: value.title,
            summary: value.summary,
            body_markdown: value.body_markdown,
            tags: value.tags,
            sources: value.sources,
            photos: value.photos,
            related: value.related,
            updated_on: value.updated_on,
            author_note: value.author_note,
            payload: CatalogPayload::Place {
                place_category: value.place_category,
                coordinates: value.coordinates,
                address: value.address,
                google_place_id: value.google_place_id,
            },
            geometry: None,
        }
    }
}

#[cfg(feature = "native")]
impl From<AuthoredRoad> for CatalogObject {
    fn from(value: AuthoredRoad) -> Self {
        Self {
            schema_version: value.schema_version,
            source_path: String::new(),
            key: ObjectKey {
                kind: ObjectKind::Road,
                id: value.id,
            },
            title: value.title,
            summary: value.summary,
            body_markdown: value.body_markdown,
            tags: value.tags,
            sources: value.sources,
            photos: value.photos,
            related: value.related,
            updated_on: value.updated_on,
            author_note: value.author_note,
            payload: CatalogPayload::Road {
                geometry_path: value.geometry_path,
                surface: value.surface,
                surface_evidence: value.surface_evidence,
            },
            geometry: None,
        }
    }
}

#[cfg(feature = "native")]
impl From<AuthoredRoute> for CatalogObject {
    fn from(value: AuthoredRoute) -> Self {
        Self {
            schema_version: value.schema_version,
            source_path: String::new(),
            key: ObjectKey {
                kind: ObjectKind::Route,
                id: value.id,
            },
            title: value.title,
            summary: value.summary,
            body_markdown: value.body_markdown,
            tags: value.tags,
            sources: value.sources,
            photos: value.photos,
            related: value.related,
            updated_on: value.updated_on,
            author_note: value.author_note,
            payload: CatalogPayload::Route {
                category: value.category,
                color: value.color,
                distance_mi: value.distance_mi,
                waypoints: value.waypoints,
                via: value.via,
                geometry_path: value.geometry_path,
                route_type: value.route_type,
                navigation_junctions: value.navigation_junctions,
                stops: value.stops,
            },
            geometry: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogIndex {
    pub schema_version: u32,
    pub objects: Vec<CatalogSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogSummary {
    pub key: ObjectKey,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub search_text: String,
    pub bounds: Bounds,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub point: Option<Coordinate>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub distance_mi: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub place_category: Option<PlaceCategory>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub route_type: Option<RouteType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_label: Option<String>,
    pub page_url: String,
    pub detail_url: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub overview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub geometry_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoVariant {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputPhoto {
    pub alt: String,
    pub credit: String,
    pub rights: String,
    pub variants: Vec<PhotoVariant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelatedObject {
    pub key: ObjectKey,
    pub title: String,
    pub page_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StopDetail {
    pub key: ObjectKey,
    pub title: String,
    pub page_url: String,
    pub coordinates: Coordinate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NearbyPlace {
    pub key: ObjectKey,
    pub title: String,
    pub page_url: String,
    pub coordinates: Coordinate,
    pub distance_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationMode {
    Destination,
    Start,
    Stage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigationLink {
    pub label: String,
    pub url: String,
    pub mode: NavigationMode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectDetail {
    pub schema_version: u32,
    pub key: ObjectKey,
    pub title: String,
    pub summary: String,
    pub body_html: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub photos: Vec<OutputPhoto>,
    #[serde(default)]
    pub related: Vec<RelatedObject>,
    #[serde(default)]
    pub stops: Vec<StopDetail>,
    #[serde(default)]
    pub nearby_places: Vec<NearbyPlace>,
    pub bounds: Bounds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<Coordinate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance_mi: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waypoints: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_type: Option<RouteType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place_category: Option<PlaceCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_note: Option<AuthorNote>,
    #[serde(default)]
    pub navigation_links: Vec<NavigationLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpx_url: Option<String>,
}
