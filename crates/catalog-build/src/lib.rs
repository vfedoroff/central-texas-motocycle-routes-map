pub mod builder;
pub mod downloads;
pub mod geometry;
pub mod index;
pub mod maps_links;
pub mod media;
pub mod pages;
pub mod places_along_route;
pub mod pwa;
pub mod staging;

pub use builder::{BuildConfig, build_site};
pub use catalog_model::{NearbyPlace, StopDetail};
pub use downloads::{generate_gpx, generate_route_geojson};
pub use geometry::{DEFAULT_OVERVIEW_TOLERANCE_M, simplify_overview};
pub use index::{build_index, build_summary, compute_bounds};
pub use maps_links::{build_stage_links, google_maps_links};
pub use media::{MAX_INPUT_BYTES, MAX_PIXELS, ProcessedMedia, process_image};
pub use pages::{
    generate_sitemap, is_safe_link, render_markdown, render_place_page, render_road_page,
    render_route_page,
};
pub use places_along_route::{RoutePlaces, route_places};
pub use staging::{StagingDirectory, validate_build_paths};
