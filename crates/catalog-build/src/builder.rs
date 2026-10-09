use anyhow::{Context, Result, bail};
use catalog_model::{
    Catalog, CatalogPayload, ObjectDetail, ObjectKind, RelatedObject, Severity, load_catalog,
    validate_catalog_with_options,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    build_index,
    geometry::{DEFAULT_OVERVIEW_TOLERANCE_M, simplify_overview},
    index::compute_bounds,
    places_along_route::route_places,
    staging::{StagingDirectory, validate_build_paths},
};

pub struct BuildConfig {
    pub root: PathBuf,
    pub out: PathBuf,

    pub environment: String, // "development" or "production"

    pub ui_dir: Option<PathBuf>,
}

const DEFAULT_PRODUCTION_URL: &str = "https://central-texas-routes-map.netlify.app";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    pub schema_version: u32,
    pub title: String,
    pub region_label: String,
    pub production_url: String,
    pub language: String,
    pub nearby_radius_m: u32,
    pub nearby_limit: usize,
    pub results_page_size: usize,
    pub analytics: AnalyticsConfig,
    pub pwa: PwaConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub measurement_id: Option<String>,
    pub production_host: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PwaConfig {
    pub enabled: bool,
}

pub fn load_site_config(root: &Path) -> Result<SiteConfig> {
    let path = root.join("config/site.json");
    let config: SiteConfig = if path.exists() {
        serde_json::from_slice(&fs::read(&path)?)?
    } else {
        serde_json::from_value(json!({
            "schema_version": 1,
            "title": "Ride Atlas",
            "region_label": "Central Texas Routes",
            "production_url": DEFAULT_PRODUCTION_URL,
            "language": "en",
            "nearby_radius_m": 2000,
            "nearby_limit": 20,
            "results_page_size": 30,
            "analytics": { "enabled": false, "measurement_id": null, "production_host": "central-texas-routes-map.netlify.app" },
            "pwa": { "enabled": true }
        }))?
    };
    if config.schema_version != 1
        || config.title.trim().is_empty()
        || config.region_label.trim().is_empty()
        || config.language.trim().is_empty()
        || config.nearby_radius_m == 0
        || config.nearby_limit == 0
        || config.results_page_size == 0
    {
        bail!("Invalid config/site.json values");
    }
    let url = url::Url::parse(&config.production_url)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("config/site.json production_url must be an HTTPS origin");
    }
    if config.analytics.production_host != url.host_str().unwrap_or_default() {
        bail!("config/site.json analytics.production_host must match production_url host");
    }
    Ok(config)
}

fn resolve_related(
    related_keys: &[catalog_model::ObjectKey],
    catalog: &Catalog,
) -> Vec<RelatedObject> {
    let mut resolved = Vec::new();
    for rk in related_keys {
        if let Some(target) = catalog.objects.iter().find(|o| o.key == *rk) {
            let kind_str = match target.key.kind {
                ObjectKind::Route => "routes",
                ObjectKind::Road => "roads",
                ObjectKind::Place => "places",
            };
            resolved.push(RelatedObject {
                key: target.key.clone(),
                title: target.title.clone(),
                page_url: format!("/{}/{}/index.html", kind_str, target.key.id),
            });
        }
    }
    resolved
}

fn process_photos(
    photos: &[catalog_model::Photo],
    canonical_root: &Path,
    staging_root: &Path,
) -> Result<Vec<catalog_model::OutputPhoto>> {
    let mut output_photos = Vec::new();
    if photos.is_empty() {
        return Ok(output_photos);
    }
    fs::create_dir_all(staging_root.join("media"))?;
    for photo in photos {
        let photo_path = canonical_root.join(&photo.path);
        if !photo_path.exists() {
            bail!("Photo file not found: '{}'", photo_path.display());
        }
        let bytes = fs::read(&photo_path)?;
        let processed = crate::media::process_image(photo, &bytes)?;
        for v in &processed.variants {
            fs::write(staging_root.join(format!("media/{}", v.filename)), &v.bytes)?;
        }
        output_photos.push(processed.output_photo);
    }
    Ok(output_photos)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            bail!(
                "UI assets must not contain symlinks: {}",
                entry.path().display()
            );
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if ty.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn build_site(build_config: &BuildConfig) -> Result<()> {
    let (canonical_root, target_out) = validate_build_paths(&build_config.root, &build_config.out)?;
    let site_config = load_site_config(&canonical_root)?;
    let production_url = site_config.production_url.trim_end_matches('/');

    if build_config.environment != "development" && build_config.environment != "production" {
        bail!("Build environment must be development or production");
    }
    let is_production = build_config.environment == "production";

    // 1. Load catalog
    let catalog = match load_catalog(&canonical_root) {
        Ok(c) => c,
        Err(diags) => {
            for d in &diags {
                eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
            }
            bail!("Failed to load catalog with {} error(s)", diags.len());
        }
    };

    // 2. Schema and topology validation
    let schema_diags = validate_catalog_with_options(&catalog, false);
    let has_errors = schema_diags.iter().any(|d| d.severity == Severity::Error);
    if has_errors {
        for d in &schema_diags {
            if d.severity == Severity::Error {
                eprintln!("[{:?}] {}: {} ({})", d.severity, d.code, d.message, d.file);
            }
        }
        bail!("Catalog validation failed with schema/topology error(s)");
    }

    // 4. Staging transaction
    let staging = StagingDirectory::new(&target_out)?;
    let staging_root = &staging.staging_path;

    // Create delivery directories
    fs::create_dir_all(staging_root.join("data/routes"))?;
    fs::create_dir_all(staging_root.join("data/roads"))?;
    fs::create_dir_all(staging_root.join("data/places"))?;
    fs::create_dir_all(staging_root.join("data/overview/routes"))?;
    fs::create_dir_all(staging_root.join("data/overview/roads"))?;
    fs::create_dir_all(staging_root.join("data/geometry/routes"))?;
    fs::create_dir_all(staging_root.join("data/geometry/roads"))?;
    fs::create_dir_all(staging_root.join("downloads/routes"))?;

    // 5. Build and write /catalog-index.json
    let index = build_index(&catalog);
    fs::write(
        staging_root.join("catalog-index.json"),
        serde_json::to_string(&index)?,
    )?;

    // 6. Generate details and geometries for each object
    for obj in &catalog.objects {
        match &obj.payload {
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
                let bounds = compute_bounds(coords);

                let mut route_props = serde_json::Map::new();
                route_props.insert("color".into(), json!(color));
                route_props.insert("title".into(), json!(obj.title));

                // Authoritative geometry Feature
                let authoritative_feat = json!({
                    "type": "Feature",
                    "properties": route_props.clone(),
                    "geometry": {
                        "type": "LineString",
                        "coordinates": coords
                    }
                });
                fs::write(
                    staging_root.join(format!("data/geometry/routes/{}.geojson", obj.key.id)),
                    serde_json::to_string(&authoritative_feat)?,
                )?;

                // Overview geometry Feature
                let overview_coords = simplify_overview(coords, DEFAULT_OVERVIEW_TOLERANCE_M);
                let overview_feat = json!({
                    "type": "Feature",
                    "properties": route_props,
                    "geometry": {
                        "type": "LineString",
                        "coordinates": overview_coords
                    }
                });
                fs::write(
                    staging_root.join(format!("data/overview/routes/{}.geojson", obj.key.id)),
                    serde_json::to_string(&overview_feat)?,
                )?;

                // Downloads: GPX 1.1 and standalone GeoJSON
                let gpx_content = crate::downloads::generate_gpx(&obj.title, coords);
                fs::write(
                    staging_root.join(format!("downloads/routes/{}.gpx", obj.key.id)),
                    gpx_content,
                )?;
                let download_geojson = crate::downloads::generate_route_geojson(coords);
                fs::write(
                    staging_root.join(format!("downloads/routes/{}.geojson", obj.key.id)),
                    serde_json::to_string(&download_geojson)?,
                )?;

                // Resolve stops & nearby places
                let places = route_places(obj, &catalog);
                let related = resolve_related(&obj.related, &catalog);
                let photos = process_photos(&obj.photos, &canonical_root, staging_root)?;

                let start_label = navigation_junctions
                    .first()
                    .filter(|j| j.coordinates.is_some() && !j.label.trim().is_empty())
                    .map(|j| j.label.clone());

                let detail = ObjectDetail {
                    schema_version: 1,
                    key: obj.key.clone(),
                    title: obj.title.clone(),
                    summary: obj.summary.clone(),
                    body_html: crate::pages::render_markdown(&obj.body_markdown),
                    tags: obj.tags.clone(),
                    sources: obj.sources.clone(),
                    photos,
                    related,
                    stops: places.stops,
                    nearby_places: places.nearby_places,
                    bounds,
                    coordinates: None,
                    category: Some(category.clone()),
                    color: Some(color.clone()),
                    distance_mi: Some(*distance_mi),
                    waypoints: Some(waypoints.clone()),
                    via: Some(via.clone()),
                    route_type: Some(*route_type),
                    start_label,
                    place_category: None,
                    address: None,
                    author_note: obj.author_note.clone(),
                    navigation_links: crate::maps_links::google_maps_links(obj),
                    geometry_url: Some(format!("/data/geometry/routes/{}.geojson", obj.key.id)),
                    gpx_url: Some(format!("/downloads/routes/{}.gpx", obj.key.id)),
                };

                fs::write(
                    staging_root.join(format!("data/routes/{}.json", obj.key.id)),
                    serde_json::to_string(&detail)?,
                )?;

                // Static HTML page
                let social_dir = staging_root.join("social/routes");
                fs::create_dir_all(&social_dir)?;
                crate::social::render_route_image(
                    coords,
                    color,
                    &obj.title,
                    *distance_mi,
                    &social_dir.join(format!("{}.png", obj.key.id)),
                    &canonical_root.join(".cache/social-maps"),
                )
                .with_context(|| {
                    format!(
                        "Failed to generate geographic preview for route {}",
                        obj.key.id
                    )
                })?;
                let page_dir = staging_root.join(format!("routes/{}", obj.key.id));
                fs::create_dir_all(&page_dir)?;
                let page_html = crate::pages::render_route_page_with_preview(
                    &detail,
                    production_url,
                    build_config.environment == "development",
                )?;
                fs::write(page_dir.join("index.html"), page_html)?;
            }
            CatalogPayload::Road { .. } => {
                let coords = obj.geometry.as_deref().unwrap_or(&[]);
                let bounds = compute_bounds(coords);

                let mut road_props = serde_json::Map::new();
                road_props.insert("title".into(), json!(obj.title));

                let authoritative_feat = json!({
                    "type": "Feature",
                    "properties": road_props.clone(),
                    "geometry": {
                        "type": "LineString",
                        "coordinates": coords
                    }
                });
                fs::write(
                    staging_root.join(format!("data/geometry/roads/{}.geojson", obj.key.id)),
                    serde_json::to_string(&authoritative_feat)?,
                )?;

                let overview_coords = simplify_overview(coords, DEFAULT_OVERVIEW_TOLERANCE_M);
                let overview_feat = json!({
                    "type": "Feature",
                    "properties": road_props,
                    "geometry": {
                        "type": "LineString",
                        "coordinates": overview_coords
                    }
                });
                fs::write(
                    staging_root.join(format!("data/overview/roads/{}.geojson", obj.key.id)),
                    serde_json::to_string(&overview_feat)?,
                )?;

                let related = resolve_related(&obj.related, &catalog);
                let photos = process_photos(&obj.photos, &canonical_root, staging_root)?;

                let detail = ObjectDetail {
                    schema_version: 1,
                    key: obj.key.clone(),
                    title: obj.title.clone(),
                    summary: obj.summary.clone(),
                    body_html: crate::pages::render_markdown(&obj.body_markdown),
                    tags: obj.tags.clone(),
                    sources: obj.sources.clone(),
                    photos,
                    related,
                    stops: Vec::new(),
                    nearby_places: Vec::new(),
                    bounds,
                    coordinates: None,
                    category: None,
                    color: None,
                    distance_mi: None,
                    waypoints: None,
                    via: None,
                    route_type: None,
                    start_label: None,
                    place_category: None,
                    address: None,
                    author_note: obj.author_note.clone(),
                    navigation_links: crate::maps_links::google_maps_links(obj),
                    geometry_url: Some(format!("/data/geometry/roads/{}.geojson", obj.key.id)),
                    gpx_url: None,
                };

                fs::write(
                    staging_root.join(format!("data/roads/{}.json", obj.key.id)),
                    serde_json::to_string(&detail)?,
                )?;

                // Static HTML page
                let page_dir = staging_root.join(format!("roads/{}", obj.key.id));
                fs::create_dir_all(&page_dir)?;
                let page_html = crate::pages::render_road_page_with_preview(
                    &detail,
                    production_url,
                    build_config.environment == "development",
                )?;
                fs::write(page_dir.join("index.html"), page_html)?;
            }
            CatalogPayload::Place {
                place_category,
                coordinates,
                address,
                ..
            } => {
                let bounds = [
                    coordinates[0],
                    coordinates[1],
                    coordinates[0],
                    coordinates[1],
                ];
                let related = resolve_related(&obj.related, &catalog);
                let photos = process_photos(&obj.photos, &canonical_root, staging_root)?;

                let detail = ObjectDetail {
                    schema_version: 1,
                    key: obj.key.clone(),
                    title: obj.title.clone(),
                    summary: obj.summary.clone(),
                    body_html: crate::pages::render_markdown(&obj.body_markdown),
                    tags: obj.tags.clone(),
                    sources: obj.sources.clone(),
                    photos,
                    related,
                    stops: Vec::new(),
                    nearby_places: Vec::new(),
                    bounds,
                    coordinates: Some(*coordinates),
                    category: None,
                    color: None,
                    distance_mi: None,
                    waypoints: None,
                    via: None,
                    route_type: None,
                    start_label: None,
                    place_category: Some(*place_category),
                    address: address.clone(),
                    author_note: obj.author_note.clone(),
                    navigation_links: crate::maps_links::google_maps_links(obj),
                    geometry_url: None,
                    gpx_url: None,
                };

                fs::write(
                    staging_root.join(format!("data/places/{}.json", obj.key.id)),
                    serde_json::to_string(&detail)?,
                )?;

                // Static HTML page
                let page_dir = staging_root.join(format!("places/{}", obj.key.id));
                fs::create_dir_all(&page_dir)?;
                let page_html = crate::pages::render_place_page_with_preview(
                    &detail,
                    production_url,
                    build_config.environment == "development",
                )?;
                fs::write(page_dir.join("index.html"), page_html)?;
            }
        }
    }

    // 7. Generate sitemap.xml
    let sitemap = crate::pages::generate_sitemap(production_url, &catalog.objects);
    fs::write(staging_root.join("sitemap.xml"), sitemap)?;

    // 8. Generate privacy/index.html
    let privacy_dir = staging_root.join("privacy");
    fs::create_dir_all(&privacy_dir)?;
    let privacy_html = crate::pages::render_privacy_page(
        production_url,
        build_config.environment == "development",
    )?;
    fs::write(privacy_dir.join("index.html"), privacy_html)?;

    // 9. Prepare client config (force analytics off in non-production builds)
    let effective_analytics = if is_production {
        site_config.analytics.clone()
    } else {
        AnalyticsConfig {
            enabled: false,
            measurement_id: None,
            production_host: site_config.analytics.production_host.clone(),
        }
    };
    let client_config = json!({
        "environment": build_config.environment,
        "analytics": effective_analytics,
    });
    let config_json = serde_json::to_string(&client_config)?;
    fs::write(staging_root.join("data/site-config.json"), &config_json)?;

    // 10. Compose UI assets if ui_dir provided
    if let Some(ui_dir) = &build_config.ui_dir {
        if ui_dir.exists() {
            if ui_dir.is_symlink() || !ui_dir.canonicalize()?.starts_with(&canonical_root) {
                bail!("UI asset directory must be inside the build root and not a symlink");
            }
            compose_ui(
                ui_dir,
                staging_root,
                build_config.environment == "development",
                &config_json,
            )?;
        } else {
            bail!("Provided ui_dir does not exist: '{}'", ui_dir.display());
        }
    }

    // 11. Generate PWA assets and offline route packs if PWA is enabled
    if site_config.pwa.enabled {
        crate::pwa::generate_pwa_icons(staging_root)?;
        crate::pwa::generate_manifest(staging_root, &site_config.title)?;
        let packs = crate::pwa::generate_offline_route_packs(&catalog, staging_root)?;
        let site_version = crate::pwa::compute_site_version(staging_root)?;
        let shell_urls =
            crate::pwa::generate_global_offline_manifest(staging_root, &site_version, packs)?;
        crate::pwa::generate_service_worker(staging_root, &site_version, &shell_urls)?;
    }

    // 12. Commit atomic replacement
    staging.commit()?;

    Ok(())
}

fn extract_head_assets(html: &str) -> Result<String> {
    let head_start = html
        .find("<head>")
        .or_else(|| html.find("<HEAD>"))
        .map(|pos| pos + 6)
        .context("Missing <head> in Trunk index.html")?;
    let head_end = html
        .find("</head>")
        .or_else(|| html.find("</HEAD>"))
        .context("Missing </head> in Trunk index.html")?;
    let head_content = &html[head_start..head_end];

    // Filter out title, common meta tags, and catalog-config script provided by root template
    let mut assets = Vec::new();
    for line in head_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("<meta ")
            || trimmed.starts_with("<title>")
            || trimmed.starts_with("</title>")
            || trimmed.starts_with("<script id=\"catalog-config\"")
        {
            continue;
        }
        assets.push(line);
    }
    Ok(assets.join("\n"))
}

fn extract_referenced_asset_urls(head_assets: &str) -> Result<Vec<String>> {
    let mut urls = Vec::new();
    let mut rest = head_assets;
    while let Some(pos) = rest.find("/app/") {
        let slice = &rest[pos..];
        let end = slice
            .find(|c: char| {
                c == '"' || c == '\'' || c == ' ' || c == '\n' || c == '\r' || c == '>' || c == ';'
            })
            .unwrap_or(slice.len());
        let url = &slice[..end];
        validate_asset_url(url)?;
        if !urls.contains(&url.to_string()) {
            urls.push(url.to_string());
        }
        rest = &slice[end..];
    }
    urls.sort();
    Ok(urls)
}

fn validate_asset_url(url: &str) -> Result<()> {
    let Some(relative) = url.strip_prefix("/app/") else {
        bail!("UI asset URL must begin with /app/: {url}");
    };
    if relative.is_empty()
        || relative.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        })
    {
        bail!("Unsafe UI asset URL: {url}");
    }
    Ok(())
}

fn validate_head_asset_attributes(head_assets: &str) -> Result<()> {
    for attribute in ["src", "href"] {
        for quote in ['\'', '"'] {
            let needle = format!("{attribute}={quote}");
            let mut rest = head_assets;
            while let Some(offset) = rest.find(&needle) {
                let before = &rest[..offset];
                if before
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
                {
                    rest = &rest[offset + needle.len()..];
                    continue;
                }
                let value_start = &rest[offset + needle.len()..];
                let end = value_start
                    .find(quote)
                    .context("Unterminated Trunk asset attribute")?;
                validate_asset_url(&value_start[..end])?;
                rest = &value_start[end + 1..];
            }
        }
    }
    Ok(())
}

fn compose_ui(
    ui_dir: &Path,
    staging_root: &Path,
    development_preview: bool,
    config_json: &str,
) -> Result<()> {
    let index_file = ui_dir.join("index.html");
    if !index_file.is_file() || index_file.is_symlink() {
        bail!(
            "Trunk index.html not found in ui_dir: '{}'",
            ui_dir.display()
        );
    }

    let index_content = fs::read_to_string(&index_file)?;
    let head_assets = extract_head_assets(&index_content)?;
    validate_head_asset_attributes(&head_assets)?;
    let referenced_urls = extract_referenced_asset_urls(&head_assets)?;

    if referenced_urls.is_empty() {
        bail!("No UI asset references found in Trunk index.html");
    }

    // Copy UI assets into staging_root/app/
    let app_dir = staging_root.join("app");
    fs::create_dir_all(&app_dir)?;

    for entry in fs::read_dir(ui_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str != "index.html" {
            let ty = entry.file_type()?;
            if ty.is_symlink() {
                bail!(
                    "UI assets must not contain symlinks: {}",
                    entry.path().display()
                );
            }
            if name_str == "manifest.json" {
                bail!("Trunk output must not supply app/manifest.json");
            }
            let dest = app_dir.join(&name);
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &dest)?;
            } else if ty.is_file() {
                fs::copy(entry.path(), dest)?;
            }
        }
    }

    // Verify all referenced UI asset URLs exist in staging_root/app/
    for url in &referenced_urls {
        if let Some(rel) = url.strip_prefix("/app/") {
            let asset_path = app_dir.join(rel);
            if !asset_path.exists() {
                bail!("Referenced UI asset URL does not exist on disk: {}", url);
            }
            if !asset_path.is_file() || asset_path.is_symlink() {
                bail!("Referenced UI asset URL is not a regular file: {}", url);
            }
        }
    }
    if !referenced_urls.iter().any(|url| url.ends_with(".js"))
        || !referenced_urls.iter().any(|url| url.ends_with(".wasm"))
        || !referenced_urls.iter().any(|url| url.ends_with(".css"))
    {
        bail!("Trunk index.html must reference JS, WASM, and CSS assets");
    }

    let manifest = json!({
        "schema_version": 1,
        "css": referenced_urls.iter().filter(|url| url.ends_with(".css")).collect::<Vec<_>>(),
        "scripts": referenced_urls.iter().filter(|url| url.ends_with(".js")).collect::<Vec<_>>(),
        "wasm": referenced_urls.iter().filter(|url| url.ends_with(".wasm")).collect::<Vec<_>>(),
        "assets": referenced_urls,
    });
    fs::write(
        app_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    // Render the root map template into staging_root/index.html
    let root_html = crate::pages::render_root_map(&head_assets, development_preview, config_json)?;
    fs::write(staging_root.join("index.html"), root_html)?;

    Ok(())
}
