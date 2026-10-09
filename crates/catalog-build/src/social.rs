//! Social previews use the public-domain USGS National Map, without credentials.
//! Source and terms:
//! https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer
//! https://www.usgs.gov/faqs/what-are-terms-uselicensing-map-services-and-data-national-map
//! Export protocol: https://developers.arcgis.com/rest/services-reference/enterprise/export-map/
//! Background PNGs and returned EPSG:3857 extents persist in `.cache/social-maps`.
//! Preserve that directory between CI builds; remove it to refresh USGS data.
//! Title/color changes reuse backgrounds; geographic changes get a new cache identity.

use anyhow::{Context, Result, bail};
use image::{Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{fs, time::Duration};

/// Render a route over a cached USGS National Map export.
pub fn render_route_image(
    coords: &[[f64; 2]],
    color: &str,
    title: &str,
    distance: f64,
    path: &Path,
    cache_dir: &Path,
) -> Result<()> {
    let mut image = RgbImage::from_pixel(2400, 1260, Rgb([246, 248, 245]));
    let color = color.strip_prefix('#').unwrap_or(color);
    let route_color = if color.len() == 6 {
        Rgb([
            u8::from_str_radix(&color[0..2], 16).unwrap_or(60),
            u8::from_str_radix(&color[2..4], 16).unwrap_or(80),
            u8::from_str_radix(&color[4..6], 16).unwrap_or(100),
        ])
    } else {
        Rgb([60, 80, 100])
    };
    let requested = MapViewport::for_route(coords)?;
    let (background, viewport) = load_background(&requested, cache_dir)?;
    let background = image::imageops::resize(
        &background,
        MAP_WIDTH,
        MAP_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    image::imageops::replace(&mut image, &background, 0, 340);
    let projected: Vec<_> = coords
        .iter()
        .map(|&p| {
            let [x, y] = viewport.pixel(p);
            [x, y + 340.0]
        })
        .collect();
    {
        for segment in projected.windows(2) {
            stroke(
                &mut image,
                segment[0],
                segment[1],
                13.0,
                Rgb([255, 255, 255]),
            );
        }
        for segment in projected.windows(2) {
            stroke(&mut image, segment[0], segment[1], 7.0, route_color);
        }
        stroke(
            &mut image,
            projected[0],
            projected[0],
            27.0,
            Rgb([255, 255, 255]),
        );
        stroke(
            &mut image,
            projected[0],
            projected[0],
            18.0,
            Rgb([32, 46, 40]),
        );
        stroke(
            &mut image,
            projected[0],
            projected[0],
            7.0,
            Rgb([255, 255, 255]),
        );
    }
    let font = fontdue::Font::from_bytes(
        include_bytes!("../assets/fonts/Roboto.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .map_err(|e| anyhow::anyhow!(e))?;
    crate::social_places::draw_towns(
        &mut image,
        &font,
        &viewport,
        coords,
        cache_dir,
        fetch_map_asset,
    )?;
    let title_size = (105.0_f32).min(
        2100.0
            / title
                .chars()
                .map(|c| font.metrics(c, 1.0).advance_width)
                .sum::<f32>()
                .max(1.0),
    );
    draw_text(&mut image, &font, title, 150, 210, title_size);
    draw_text(
        &mut image,
        &font,
        &format!("{distance:.1} miles · Central Texas"),
        150,
        310,
        48.0,
    );
    draw_text(&mut image, &font, "RIDE ATLAS", 60, 1235, 30.0);
    draw_text(
        &mut image,
        &font,
        "Map services and data available from U.S. Geological Survey, National Geospatial Program.",
        750,
        1235,
        26.0,
    );
    image::imageops::resize(&image, 1200, 630, image::imageops::FilterType::Lanczos3).save(path)?;
    Ok(())
}

pub(crate) fn stroke(image: &mut RgbImage, a: [f64; 2], b: [f64; 2], radius: f64, color: Rgb<u8>) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length_sq = dx * dx + dy * dy;
    let left = (a[0].min(b[0]) - radius).max(0.0) as u32;
    let right = (a[0].max(b[0]) + radius).min(f64::from(image.width() - 1)) as u32;
    let top = (a[1].min(b[1]) - radius).max(0.0) as u32;
    let bottom = (a[1].max(b[1]) + radius).min(f64::from(image.height() - 1)) as u32;
    for y in top..=bottom {
        for x in left..=right {
            let px = f64::from(x) - a[0];
            let py = f64::from(y) - a[1];
            let t = if length_sq == 0.0 {
                0.0
            } else {
                ((px * dx + py * dy) / length_sq).clamp(0.0, 1.0)
            };
            if (px - t * dx).powi(2) + (py - t * dy).powi(2) <= radius * radius {
                image.put_pixel(x, y, color);
            }
        }
    }
}

fn draw_text(
    image: &mut RgbImage,
    font: &fontdue::Font,
    text: &str,
    x: i32,
    baseline: i32,
    size: f32,
) {
    draw_text_color(image, font, text, x, baseline, size, Rgb([32, 46, 40]));
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_color(
    image: &mut RgbImage,
    font: &fontdue::Font,
    text: &str,
    x: i32,
    baseline: i32,
    size: f32,
    color: Rgb<u8>,
) {
    let mut cursor = x as f32;
    for c in text.chars() {
        let (m, bitmap) = font.rasterize(c, size);
        for y in 0..m.height {
            for x in 0..m.width {
                let px = cursor as i32 + m.xmin + x as i32;
                let py = baseline - m.ymin - m.height as i32 + y as i32;
                if px >= 0 && py >= 0 && px < image.width() as i32 && py < image.height() as i32 {
                    let alpha = bitmap[y * m.width + x] as f32 / 255.0;
                    let pixel = image.get_pixel_mut(px as u32, py as u32);
                    for channel in 0..3 {
                        pixel[channel] = (pixel[channel] as f32 * (1.0 - alpha)
                            + f32::from(color[channel]) * alpha)
                            as u8;
                    }
                }
            }
        }
        cursor += m.advance_width;
    }
}

const MAP_WIDTH: u32 = 2400;
const MAP_HEIGHT: u32 = 840;
// Request at final display resolution: cached USGS cartography ignores DPI scaling,
// so a double-sized export would shrink its labels when the preview is downsampled.
const EXPORT_WIDTH: u32 = 1200;
const EXPORT_HEIGHT: u32 = 420;
const MAP_SERVICE: &str =
    "https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/export";

/// Web Mercator extent of a map export. Pixel coordinates are relative to the map panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapViewport {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
}

impl MapViewport {
    pub fn project([lon, lat]: [f64; 2]) -> [f64; 2] {
        let radius = 6_378_137.0;
        [
            radius * lon.to_radians(),
            radius
                * (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0)
                    .tan()
                    .ln(),
        ]
    }

    pub fn for_route(coords: &[[f64; 2]]) -> Result<Self> {
        if coords.is_empty()
            || coords.iter().any(|p| {
                !p[0].is_finite() || !p[1].is_finite() || p[0].abs() > 180.0 || p[1].abs() > 85.0
            })
        {
            bail!("Cannot generate a geographic preview: empty or invalid route coordinates");
        }
        let points: Vec<_> = coords.iter().copied().map(Self::project).collect();
        let min_x = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);
        // Reserve map margins without distorting its aspect ratio; degenerate routes still get context.
        let meters_per_pixel = ((max_x - min_x) / 2160.0)
            .max((max_y - min_y) / 600.0)
            .max(2.0);
        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;
        Ok(Self {
            xmin: cx - 1200.0 * meters_per_pixel,
            xmax: cx + 1200.0 * meters_per_pixel,
            ymin: cy - 420.0 * meters_per_pixel,
            ymax: cy + 420.0 * meters_per_pixel,
        })
    }

    pub fn pixel(&self, coord: [f64; 2]) -> [f64; 2] {
        let [x, y] = Self::project(coord);
        [
            (x - self.xmin) / (self.xmax - self.xmin) * f64::from(MAP_WIDTH),
            (self.ymax - y) / (self.ymax - self.ymin) * f64::from(MAP_HEIGHT),
        ]
    }

    fn request_url(&self) -> Result<url::Url> {
        let mut url = url::Url::parse(MAP_SERVICE)?;
        url.query_pairs_mut().extend_pairs([
            (
                "bbox",
                format!("{},{},{},{}", self.xmin, self.ymin, self.xmax, self.ymax),
            ),
            ("bboxSR", "3857".into()),
            ("imageSR", "3857".into()),
            ("size", format!("{EXPORT_WIDTH},{EXPORT_HEIGHT}")),
            ("dpi", "96".into()),
            ("format", "png24".into()),
            ("transparent", "false".into()),
            ("f", "json".into()),
        ]);
        Ok(url)
    }

    /// Cache identity includes the source, projection, extent, dimensions and label density.
    pub fn cache_paths(&self, dir: &Path) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
        let key: String = Sha256::digest(self.request_url()?.as_str().as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok((
            dir.join(format!("{key}.png")),
            dir.join(format!("{key}.json")),
        ))
    }
}

#[derive(Deserialize)]
struct MapExport {
    href: String,
    width: u32,
    height: u32,
    extent: MapViewport,
}

fn validate_background(bytes: &[u8], viewport: &MapViewport) -> Result<RgbImage> {
    if ![viewport.xmin, viewport.xmax, viewport.ymin, viewport.ymax]
        .iter()
        .all(|v| v.is_finite())
        || viewport.xmin >= viewport.xmax
        || viewport.ymin >= viewport.ymax
    {
        bail!("USGS map has an invalid extent");
    }
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgb8();
    if image.dimensions() != (EXPORT_WIDTH, EXPORT_HEIGHT) {
        bail!(
            "USGS map has unexpected dimensions {:?}",
            image.dimensions()
        );
    }
    // A blank service response must never masquerade as a map.
    if image.pixels().all(|p| p == image.get_pixel(0, 0)) {
        bail!("USGS returned a blank map image");
    }
    Ok(image)
}

fn fetch_map_asset(url: &str) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into();
    Ok(agent
        .get(url)
        .header("User-Agent", "RideAtlas/1.0 (static route previews)")
        .call()?
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_vec()?)
}

fn load_background(viewport: &MapViewport, dir: &Path) -> Result<(RgbImage, MapViewport)> {
    load_background_with(viewport, dir, fetch_map_asset).with_context(|| format!(
        "USGS geographic preview unavailable (cache: {}). Check network access or restore the map cache; no outline fallback was generated", dir.display()))
}

fn load_background_with(
    viewport: &MapViewport,
    dir: &Path,
    mut fetch: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<(RgbImage, MapViewport)> {
    let (image_path, extent_path) = viewport.cache_paths(dir)?;
    if image_path.exists() && extent_path.exists() {
        let cached = (|| -> Result<_> {
            let extent: MapViewport = serde_json::from_slice(&fs::read(&extent_path)?)?;
            let image = validate_background(&fs::read(&image_path)?, &extent)?;
            Ok((image, extent))
        })();
        if let Ok(background) = cached {
            return Ok(background);
        }
        eprintln!(
            "Invalid USGS preview cache {}; fetching a fresh export",
            image_path.display()
        );
    }
    eprintln!("Fetching USGS preview map for {}", image_path.display());
    let response = fetch(viewport.request_url()?.as_str())?;
    let export: MapExport = serde_json::from_slice(&response).with_context(|| {
        format!(
            "Invalid USGS export response: {}",
            String::from_utf8_lossy(&response[..response.len().min(500)])
        )
    })?;
    if (export.width, export.height) != (EXPORT_WIDTH, EXPORT_HEIGHT) {
        bail!("USGS export dimensions do not match the request");
    }
    let href = url::Url::parse(&export.href)?;
    if href.scheme() != "https" || href.host_str() != Some("basemap.nationalmap.gov") {
        bail!("USGS export returned an unexpected image host");
    }
    let bytes = fetch(href.as_str())?;
    let image = validate_background(&bytes, &export.extent)?;
    fs::create_dir_all(dir)?;
    // Publish the manifest last so interrupted downloads are never valid cache entries.
    let temp_image = image_path.with_extension("png.tmp");
    let temp_extent = extent_path.with_extension("json.tmp");
    fs::write(&temp_image, &bytes)?;
    fs::write(&temp_extent, serde_json::to_vec(&export.extent)?)?;
    fs::rename(temp_image, image_path)?;
    fs::rename(temp_extent, extent_path)?;
    Ok((image, export.extent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn map_bytes() -> Vec<u8> {
        let mut image = RgbImage::from_pixel(EXPORT_WIDTH, EXPORT_HEIGHT, Rgb([230, 235, 225]));
        image.put_pixel(600, 210, Rgb([255, 255, 255]));
        let mut out = Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn cache_reuses_map_and_preserves_server_extent() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let requested = MapViewport::for_route(&[[-98.0, 30.0], [-97.0, 31.0]])?;
        let actual = MapViewport {
            xmin: requested.xmin - 100.0,
            ..requested.clone()
        };
        let response = serde_json::to_vec(&serde_json::json!({
            "href": "https://basemap.nationalmap.gov/test.png", "width": EXPORT_WIDTH,
            "height": EXPORT_HEIGHT, "extent": actual,
        }))?;
        let mut calls = 0;
        let (_, extent) = load_background_with(&requested, dir.path(), |_| {
            calls += 1;
            Ok(if calls == 1 {
                response.clone()
            } else {
                map_bytes()
            })
        })?;
        assert_eq!(calls, 2);
        assert_eq!(extent.xmin, actual.xmin);
        let (_, cached_extent) = load_background_with(&requested, dir.path(), |_| {
            panic!("warm cache must work without network")
        })?;
        assert_eq!(cached_extent.xmin, actual.xmin);
        Ok(())
    }

    #[test]
    fn failed_fetch_never_creates_a_cache_entry() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let viewport = MapViewport::for_route(&[[-98.0, 30.0]])?;
        let result = load_background_with(&viewport, dir.path(), |_| bail!("offline"));
        assert!(result.unwrap_err().to_string().contains("offline"));
        assert_eq!(fs::read_dir(dir.path())?.count(), 0);
        Ok(())
    }

    #[test]
    fn corrupted_cache_is_refetched_and_error_responses_are_rejected() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let viewport = MapViewport::for_route(&[[-98.0, 30.0]])?;
        let (png, json) = viewport.cache_paths(dir.path())?;
        fs::write(png, b"incomplete image")?;
        fs::write(json, serde_json::to_vec(&viewport)?)?;
        let result = load_background_with(&viewport, dir.path(), |_| {
            Ok(br#"{"error":{"code":500}}"#.to_vec())
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid USGS export response")
        );
        Ok(())
    }

    #[test]
    fn blank_maps_and_invalid_extents_are_rejected() -> Result<()> {
        let extent = MapViewport::for_route(&[[-98.0, 30.0]])?;
        let image = RgbImage::from_pixel(EXPORT_WIDTH, EXPORT_HEIGHT, Rgb([255, 255, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png)?;
        assert!(
            validate_background(&bytes.into_inner(), &extent)
                .unwrap_err()
                .to_string()
                .contains("blank")
        );
        let invalid = MapViewport {
            xmax: extent.xmin,
            ..extent
        };
        assert!(
            validate_background(&map_bytes(), &invalid)
                .unwrap_err()
                .to_string()
                .contains("extent")
        );
        Ok(())
    }

    #[test]
    fn cache_identity_tracks_geography() -> Result<()> {
        let a = MapViewport::for_route(&[[-98.0, 30.0]])?;
        let b = MapViewport::for_route(&[[-99.0, 30.0]])?;
        assert_ne!(
            a.cache_paths(Path::new("cache"))?,
            b.cache_paths(Path::new("cache"))?
        );
        Ok(())
    }
}
