//! Readable town labels from USGS GNIS Populated Places, under National Map terms.
//! https://carto.nationalmap.gov/arcgis/rest/services/geonames/MapServer/3
use crate::social::{MapViewport, draw_text_color, stroke};
use anyhow::{Context, Result, bail};
use image::{Rgb, RgbImage};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const SERVICE: &str =
    "https://carto.nationalmap.gov/arcgis/rest/services/geonames/MapServer/3/query";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Town {
    name: String,
    coordinates: [f64; 2],
}

#[derive(Deserialize)]
struct TownResponse {
    features: Vec<TownFeature>,
    #[serde(rename = "exceededTransferLimit", default)]
    truncated: bool,
}

#[derive(Deserialize)]
struct TownFeature {
    attributes: TownName,
    geometry: TownPoint,
}

#[derive(Deserialize)]
struct TownName {
    gaz_name: String,
}

#[derive(Deserialize)]
struct TownPoint {
    points: Vec<[f64; 2]>,
}

fn load_towns(
    viewport: &MapViewport,
    dir: &Path,
    mut fetch: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<Town>> {
    // Bump this suffix if the GNIS source, query fields or coordinate system changes.
    let path = viewport.cache_paths(dir)?.0.with_extension("gnis-v1.json");
    if path.exists() {
        if let Ok(towns) = serde_json::from_slice::<Vec<Town>>(&fs::read(&path)?)
            && towns.iter().all(valid_town)
        {
            return Ok(towns);
        }
        eprintln!(
            "Invalid GNIS cache {}; fetching town labels",
            path.display()
        );
    }
    let mut url = url::Url::parse(SERVICE)?;
    url.query_pairs_mut().extend_pairs([
        (
            "geometry",
            format!(
                "{},{},{},{}",
                viewport.xmin, viewport.ymin, viewport.xmax, viewport.ymax
            ),
        ),
        ("geometryType", "esriGeometryEnvelope".into()),
        ("inSR", "3857".into()),
        ("outSR", "4326".into()),
        ("outFields", "gaz_name".into()),
        ("where", "gaz_name NOT LIKE '%(historical)%'".into()),
        ("spatialRel", "esriSpatialRelIntersects".into()),
        ("f", "json".into()),
    ]);
    eprintln!("Fetching USGS town labels for {}", path.display());
    let response: TownResponse =
        serde_json::from_slice(&fetch(url.as_str())?).context("Invalid USGS GNIS response")?;
    if response.truncated {
        bail!("USGS GNIS town query was truncated; refusing incomplete labels");
    }
    let mut towns = Vec::new();
    for feature in response.features {
        let coordinates = *feature
            .geometry
            .points
            .first()
            .context("USGS town is missing coordinates")?;
        let town = Town {
            name: feature.attributes.gaz_name,
            coordinates,
        };
        if !valid_town(&town) {
            bail!("USGS returned an invalid town label");
        }
        towns.push(town);
    }
    towns.sort_by(|a, b| a.name.cmp(&b.name));
    towns.dedup_by(|a, b| a.name == b.name && a.coordinates == b.coordinates);
    fs::create_dir_all(dir)?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec(&towns)?)?;
    fs::rename(temp, path)?;
    Ok(towns)
}

fn valid_town(town: &Town) -> bool {
    !town.name.trim().is_empty()
        && town.name.len() <= 480
        && town.coordinates.iter().all(|v| v.is_finite())
        && town.coordinates[0].abs() <= 180.0
        && town.coordinates[1].abs() <= 85.0
}

pub(crate) fn draw_towns(
    image: &mut RgbImage,
    font: &fontdue::Font,
    viewport: &MapViewport,
    coords: &[[f64; 2]],
    dir: &Path,
    fetch: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<()> {
    let towns =
        load_towns(viewport, dir, fetch).context("USGS geographic town labels unavailable")?;
    let route: Vec<_> = coords.iter().copied().map(|p| viewport.pixel(p)).collect();
    let mut candidates: Vec<_> = towns
        .iter()
        .map(|town| {
            let pixel = viewport.pixel(town.coordinates);
            let distance = route
                .windows(2)
                .map(|s| segment_distance(pixel, s[0], s[1]))
                .fold(f64::INFINITY, f64::min);
            (town, pixel, distance)
        })
        .collect();
    candidates.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.name.cmp(&b.0.name)));
    let mut occupied: Vec<[f64; 4]> = Vec::new();
    for (town, [x, y], distance) in candidates {
        if distance > 240.0 || !(30.0..2370.0).contains(&x) || !(30.0..810.0).contains(&y) {
            continue;
        }
        let width: f64 = town
            .name
            .chars()
            .map(|c| f64::from(font.metrics(c, 30.0).advance_width))
            .sum();
        let left = (x + 18.0).min(2370.0 - width);
        let baseline = y + 340.0 - 18.0;
        let rect = [
            left - 8.0,
            baseline - 38.0,
            left + width + 8.0,
            baseline + 12.0,
        ];
        if occupied
            .iter()
            .any(|r| rect[0] < r[2] && rect[2] > r[0] && rect[1] < r[3] && rect[3] > r[1])
        {
            continue;
        }
        // Halo and a small geographic point keep labels readable above terrain and route strokes.
        stroke(
            image,
            [x, y + 340.0],
            [x, y + 340.0],
            5.0,
            Rgb([255, 255, 255]),
        );
        stroke(
            image,
            [x, y + 340.0],
            [x, y + 340.0],
            2.5,
            Rgb([60, 66, 65]),
        );
        for dx in -2..=2 {
            for dy in -2..=2 {
                draw_text_color(
                    image,
                    font,
                    &town.name,
                    left as i32 + dx,
                    baseline as i32 + dy,
                    30.0,
                    Rgb([255, 255, 255]),
                );
            }
        }
        draw_text_color(
            image,
            font,
            &town.name,
            left as i32,
            baseline as i32,
            30.0,
            Rgb([32, 46, 40]),
        );
        occupied.push(rect);
        if occupied.len() == 14 {
            break;
        }
    }
    Ok(())
}

fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length
    }
    .clamp(0.0, 1.0);
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnis_coordinates_and_cache_survive_without_network() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let extent = MapViewport::for_route(&[[-99.76, 29.72]])?;
        let towns = load_towns(&extent, dir.path(), |_| {
            Ok(br#"{"features":[{"attributes":{"gaz_name":"Leakey"},"geometry":{"points":[[-99.76,29.72]]}}]}"#.to_vec())
        })?;
        assert_eq!(towns[0].name, "Leakey");
        assert_eq!(towns[0].coordinates, [-99.76, 29.72]);
        let cached = load_towns(&extent, dir.path(), |_| {
            panic!("warm cache must not fetch GNIS")
        })?;
        assert_eq!(cached[0].name, "Leakey");
        Ok(())
    }

    #[test]
    fn truncated_or_invalid_labels_fail_without_caching() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let extent = MapViewport::for_route(&[[-99.76, 29.72]])?;
        assert!(
            load_towns(&extent, dir.path(), |_| Ok(
                br#"{"features":[],"exceededTransferLimit":true}"#.to_vec()
            ))
            .is_err()
        );
        assert!(
            load_towns(&extent, dir.path(), |_| Ok(
                br#"{"error":{"code":500}}"#.to_vec()
            ))
            .is_err()
        );
        assert_eq!(fs::read_dir(dir.path())?.count(), 0);
        Ok(())
    }
}
