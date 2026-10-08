use anyhow::{Context, Result};
use catalog_model::{Catalog, CatalogPayload, ObjectKey};
use image::{ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt::Write, fs, io::Cursor, path::Path};

fn to_hex(bytes: impl AsRef<[u8]>) -> String {
    let mut s = String::with_capacity(64);
    for b in bytes.as_ref() {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OfflineResource {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteOfflinePack {
    pub schema_version: u32,
    pub key: ObjectKey,
    pub pack_version: String,
    pub resources: Vec<OfflineResource>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShellResource {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OfflinePackRef {
    pub key: ObjectKey,
    pub manifest_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalOfflineManifest {
    pub schema_version: u32,
    pub site_version: String,
    pub shell: Vec<ShellResource>,
    pub packs: Vec<OfflinePackRef>,
}

pub fn generate_brand_icon(size: u32, maskable: bool) -> Result<Vec<u8>> {
    let mut img = RgbaImage::new(size, size);
    let bg_color = Rgba([15, 23, 42, 255]); // #0f172a slate-900
    let road_color = Rgba([51, 65, 85, 255]); // #334155 slate-700
    let road_border = Rgba([100, 116, 139, 255]); // #64748b slate-500
    let line_color = Rgba([245, 158, 11, 255]); // #f59e0b amber-500

    let center = size as f64 / 2.0;
    let radius = if maskable {
        size as f64 * 0.38
    } else {
        size as f64 * 0.44
    };

    let road_width = size as f64 * 0.14;
    let line_width = size as f64 * 0.025;

    for y in 0..size {
        for x in 0..size {
            let px = x as f64;
            let py = y as f64;

            // Background fill
            img.put_pixel(x, y, bg_color);

            // Normalized coordinates relative to center [-1.0, 1.0]
            let nx = (px - center) / radius;
            let ny = (py - center) / radius;

            if nx.abs() <= 1.0 && ny.abs() <= 1.0 {
                // S-curve road: ny_road = 0.5 * sin(nx * PI)
                let expected_ny = 0.55 * (nx * std::f64::consts::PI).sin();
                let dist_to_road = ((ny - expected_ny) * radius).abs();

                if dist_to_road <= road_width + 2.0 {
                    if dist_to_road <= road_width {
                        img.put_pixel(x, y, road_color);
                        // Centerline (dashed or solid)
                        if dist_to_road <= line_width {
                            img.put_pixel(x, y, line_color);
                        }
                    } else {
                        img.put_pixel(x, y, road_border);
                    }
                }
            }
        }
    }

    let mut buf = Vec::new();
    let encoder = PngEncoder::new(Cursor::new(&mut buf));
    encoder.write_image(img.as_raw(), size, size, image::ExtendedColorType::Rgba8)?;

    Ok(buf)
}

pub fn generate_pwa_icons(staging_root: &Path) -> Result<()> {
    let icons_dir = staging_root.join("icons");
    fs::create_dir_all(&icons_dir)?;

    let icon_192 = generate_brand_icon(192, false)?;
    fs::write(icons_dir.join("icon-192.png"), icon_192)?;

    let icon_512 = generate_brand_icon(512, false)?;
    fs::write(icons_dir.join("icon-512.png"), icon_512)?;

    let icon_maskable_192 = generate_brand_icon(192, true)?;
    fs::write(icons_dir.join("icon-maskable-192.png"), icon_maskable_192)?;

    let icon_maskable_512 = generate_brand_icon(512, true)?;
    fs::write(icons_dir.join("icon-maskable-512.png"), icon_maskable_512)?;

    Ok(())
}

pub fn generate_manifest(staging_root: &Path, site_title: &str) -> Result<()> {
    let manifest = json!({
        "name": site_title,
        "short_name": site_title,
        "start_url": "/",
        "scope": "/",
        "display": "standalone",
        "theme_color": "#1e293b",
        "background_color": "#0f172a",
        "icons": [
            {
                "src": "/icons/icon-192.png",
                "sizes": "192x192",
                "type": "image/png",
                "purpose": "any"
            },
            {
                "src": "/icons/icon-512.png",
                "sizes": "512x512",
                "type": "image/png",
                "purpose": "any"
            },
            {
                "src": "/icons/icon-maskable-192.png",
                "sizes": "192x192",
                "type": "image/png",
                "purpose": "maskable"
            },
            {
                "src": "/icons/icon-maskable-512.png",
                "sizes": "512x512",
                "type": "image/png",
                "purpose": "maskable"
            }
        ]
    });

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    fs::write(staging_root.join("manifest.webmanifest"), manifest_json)?;
    Ok(())
}

fn file_sha256(path: &Path) -> Result<(String, u64)> {
    let bytes = fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = to_hex(hasher.finalize());
    Ok((hash, bytes.len() as u64))
}

pub fn generate_offline_route_packs(
    catalog: &Catalog,
    staging_root: &Path,
) -> Result<Vec<OfflinePackRef>> {
    let offline_dir = staging_root.join("data/offline/routes");
    fs::create_dir_all(&offline_dir)?;

    let mut pack_refs = Vec::new();

    for obj in &catalog.objects {
        if let CatalogPayload::Route { stops, .. } = &obj.payload {
            let id = &obj.key.id;
            let mut resource_urls: Vec<String> = Vec::new();

            // 1. Core route resources
            resource_urls.push(format!("/routes/{}/index.html", id));
            resource_urls.push(format!("/data/routes/{}.json", id));
            resource_urls.push(format!("/data/geometry/routes/{}.geojson", id));
            resource_urls.push(format!("/downloads/routes/{}.gpx", id));

            // 2. Photos on the route: 960px or largest smaller variant
            if !obj.photos.is_empty() {
                let photo_detail_path = staging_root.join(format!("data/routes/{}.json", id));
                if let Ok(content) = fs::read_to_string(&photo_detail_path)
                    && let Ok(detail) =
                        serde_json::from_str::<catalog_model::ObjectDetail>(&content)
                {
                    for out_photo in &detail.photos {
                        if let Some(best) = choose_offline_photo_variant(&out_photo.variants) {
                            resource_urls.push(best.url.clone());
                        }
                    }
                }
            }

            // 3. Authored stops (places): page, detail, photo variants
            for stop in stops {
                let stop_id = &stop.place_id;
                resource_urls.push(format!("/places/{}/index.html", stop_id));
                resource_urls.push(format!("/data/places/{}.json", stop_id));

                let stop_detail_path = staging_root.join(format!("data/places/{}.json", stop_id));
                if let Ok(content) = fs::read_to_string(&stop_detail_path)
                    && let Ok(detail) =
                        serde_json::from_str::<catalog_model::ObjectDetail>(&content)
                {
                    for out_photo in &detail.photos {
                        if let Some(best) = choose_offline_photo_variant(&out_photo.variants) {
                            resource_urls.push(best.url.clone());
                        }
                    }
                }
            }

            // Deduplicate URLs
            let mut unique_urls: Vec<String> = HashSet::<String>::from_iter(resource_urls)
                .into_iter()
                .collect();
            unique_urls.sort();

            let mut resources = Vec::new();
            for url in unique_urls {
                let rel = url.strip_prefix('/').unwrap_or(&url);
                let file_path = staging_root.join(rel);
                if file_path.exists() {
                    let (sha256, size_bytes) = file_sha256(&file_path)?;
                    resources.push(OfflineResource {
                        url,
                        sha256,
                        size_bytes,
                        required: true,
                    });
                }
            }

            // Pack version: hash of sorted resource metadata
            let mut pack_hasher = Sha256::new();
            for res in &resources {
                pack_hasher
                    .update(format!("{}:{}:{}\n", res.url, res.sha256, res.size_bytes).as_bytes());
            }
            let pack_version = to_hex(pack_hasher.finalize());

            let pack = RouteOfflinePack {
                schema_version: 1,
                key: obj.key.clone(),
                pack_version,
                resources,
            };

            let pack_file = offline_dir.join(format!("{}.json", id));
            fs::write(&pack_file, serde_json::to_string_pretty(&pack)?)?;

            pack_refs.push(OfflinePackRef {
                key: obj.key.clone(),
                manifest_url: format!("/data/offline/routes/{}.json", id),
            });
        }
    }

    pack_refs.sort_by(|a, b| a.key.id.cmp(&b.key.id));
    Ok(pack_refs)
}

fn choose_offline_photo_variant(
    variants: &[catalog_model::PhotoVariant],
) -> Option<&catalog_model::PhotoVariant> {
    if variants.is_empty() {
        return None;
    }
    // Preferred <= 960 width with maximum width
    let smaller_or_equal: Vec<_> = variants.iter().filter(|v| v.width <= 960).collect();
    if !smaller_or_equal.is_empty() {
        smaller_or_equal.into_iter().max_by_key(|v| v.width)
    } else {
        // If all are > 960, pick the smallest one
        variants.iter().min_by_key(|v| v.width)
    }
}

pub fn compute_site_version(staging_root: &Path) -> Result<String> {
    let mut file_entries = Vec::new();
    collect_files_recursive(staging_root, staging_root, &mut file_entries)?;

    // Exclude service-worker.js and offline-manifest.json
    file_entries
        .retain(|(path, _)| path != "/service-worker.js" && path != "/offline-manifest.json");

    file_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut site_hasher = Sha256::new();
    for (rel_path, sha256) in file_entries {
        site_hasher.update(format!("{}:{}\n", rel_path, sha256).as_bytes());
    }
    Ok(to_hex(site_hasher.finalize()))
}

fn collect_files_recursive(
    dir: &Path,
    root: &Path,
    entries: &mut Vec<(String, String)>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, root, entries)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root)?;
            let rel_str = format!("/{}", rel.to_slash_lossy());
            let (hash, _) = file_sha256(&path)?;
            entries.push((rel_str, hash));
        }
    }
    Ok(())
}

trait ToSlashLossy {
    fn to_slash_lossy(&self) -> String;
}

impl ToSlashLossy for Path {
    fn to_slash_lossy(&self) -> String {
        self.to_string_lossy().replace('\\', "/")
    }
}

pub fn generate_global_offline_manifest(
    staging_root: &Path,
    site_version: &str,
    packs: Vec<OfflinePackRef>,
) -> Result<Vec<String>> {
    let mut shell_resources = Vec::new();
    let mut shell_urls = Vec::new();

    // Collect shell files
    // 1. Root and index
    if staging_root.join("index.html").exists() {
        let (hash, size) = file_sha256(&staging_root.join("index.html"))?;
        shell_resources.push(ShellResource {
            url: "/".to_string(),
            sha256: hash.clone(),
            size_bytes: size,
        });
        shell_resources.push(ShellResource {
            url: "/index.html".to_string(),
            sha256: hash,
            size_bytes: size,
        });
        shell_urls.push("/".to_string());
        shell_urls.push("/index.html".to_string());
    }

    // 2. Specific shell metadata files
    let shell_candidates = [
        "/catalog-index.json",
        "/manifest.webmanifest",
        "/data/site-config.json",
        "/privacy/index.html",
        "/icons/icon-192.png",
        "/icons/icon-512.png",
        "/icons/icon-maskable-192.png",
        "/icons/icon-maskable-512.png",
    ];

    for rel_url in shell_candidates {
        let path = staging_root.join(rel_url.strip_prefix('/').unwrap());
        if path.exists() {
            let (hash, size) = file_sha256(&path)?;
            shell_resources.push(ShellResource {
                url: rel_url.to_string(),
                sha256: hash,
                size_bytes: size,
            });
            shell_urls.push(rel_url.to_string());
        }
    }

    // 3. All files in app/
    let app_dir = staging_root.join("app");
    if app_dir.exists() {
        let mut app_files = Vec::new();
        collect_app_files(&app_dir, staging_root, &mut app_files)?;
        for (url, hash, size) in app_files {
            shell_resources.push(ShellResource {
                url: url.clone(),
                sha256: hash,
                size_bytes: size,
            });
            shell_urls.push(url);
        }
    }

    shell_resources.sort_by(|a, b| a.url.cmp(&b.url));
    shell_resources.dedup_by(|a, b| a.url == b.url);

    shell_urls.sort();
    shell_urls.dedup();

    let manifest = GlobalOfflineManifest {
        schema_version: 1,
        site_version: site_version.to_string(),
        shell: shell_resources,
        packs,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(staging_root.join("offline-manifest.json"), json)?;

    Ok(shell_urls)
}

fn collect_app_files(
    dir: &Path,
    root: &Path,
    entries: &mut Vec<(String, String, u64)>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_app_files(&path, root, entries)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root)?;
            let rel_str = format!("/{}", rel.to_slash_lossy());
            let (hash, size) = file_sha256(&path)?;
            entries.push((rel_str, hash, size));
        }
    }
    Ok(())
}

pub fn generate_service_worker(
    staging_root: &Path,
    site_version: &str,
    shell_urls: &[String],
) -> Result<()> {
    let shell_urls_json = serde_json::to_string_pretty(shell_urls)?;

    let sw_code = format!(
        r#"// Ride Atlas Service Worker
// Version: {site_version}

const SITE_VERSION = "{site_version}";
const SHELL_CACHE = `ride-atlas-shell-${{SITE_VERSION}}`;

const SHELL_URLS = {shell_urls_json};

self.addEventListener("install", (event) => {{
  event.waitUntil(
    caches.open(SHELL_CACHE).then((cache) => {{
      return cache.addAll(SHELL_URLS);
    }})
  );
}});

self.addEventListener("activate", (event) => {{
  event.waitUntil(
    caches.keys().then((keys) => {{
      return Promise.all(
        keys.map((key) => {{
          if (key.startsWith("ride-atlas-shell-") && key !== SHELL_CACHE) {{
            return caches.delete(key);
          }}
          return Promise.resolve();
        }})
      );
    }}).then(() => self.clients.claim())
  );
}});

self.addEventListener("message", (event) => {{
  if (event.data && event.data.type === "SKIP_WAITING") {{
    self.skipWaiting();
  }}
}});

self.addEventListener("fetch", (event) => {{
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);

  // Exclude external origins, analytics, and third-party map tiles
  if (url.origin !== self.location.origin) return;
  if (
    url.pathname.includes("google-analytics") ||
    url.pathname.includes("googletagmanager") ||
    url.pathname.includes("openstreetmap") ||
    url.pathname.includes("tile")
  ) {{
    return;
  }}

  // Navigation mode (HTML pages): try network first, then cache, fallback to /
  if (req.mode === "navigate") {{
    event.respondWith(
      fetch(req)
        .then((response) => {{
          if (response && response.status === 200) {{
            const copy = response.clone();
            caches.open(SHELL_CACHE).then((cache) => cache.put(req, copy));
          }}
          return response;
        }})
        .catch(() => {{
          return caches.match(req).then((cached) => {{
            if (cached) return cached;
            return caches.match("/");
          }});
        }})
    );
    return;
  }}

  // Immutable hashed assets in /app/: cache-first
  if (url.pathname.startsWith("/app/")) {{
    event.respondWith(
      caches.match(req).then((cached) => {{
        if (cached) return cached;
        return fetch(req).then((response) => {{
          if (response && response.status === 200) {{
            const copy = response.clone();
            caches.open(SHELL_CACHE).then((cache) => cache.put(req, copy));
          }}
          return response;
        }});
      }})
    );
    return;
  }}

  // Data, manifests, downloads, and other assets: network-first with cache fallback
  event.respondWith(
    fetch(req, {{ cache: "no-cache" }})
      .then((response) => {{
        if (response && response.status === 200 && SHELL_URLS.includes(url.pathname)) {{
          const copy = response.clone();
          caches.open(SHELL_CACHE).then((cache) => cache.put(req, copy));
        }}
        return response;
      }})
      .catch(() => caches.match(req))
  );
}});
"#
    );

    fs::write(staging_root.join("service-worker.js"), sw_code)?;
    Ok(())
}
