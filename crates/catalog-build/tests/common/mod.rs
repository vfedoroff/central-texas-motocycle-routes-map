use anyhow::Result;
use catalog_build::social::MapViewport;
use image::{Rgb, RgbImage};
use std::{fs, path::Path};

/// Seed deterministic map assets so build tests never depend on USGS or the network.
pub fn seed_map_cache(root: &Path) -> Result<()> {
    let dir = root.join(".cache/social-maps");
    fs::create_dir_all(&dir)?;
    for entry in fs::read_dir(root.join("content/geometry/routes"))? {
        let geometry: serde_json::Value = serde_json::from_slice(&fs::read(entry?.path())?)?;
        let coords: Vec<[f64; 2]> =
            serde_json::from_value(geometry["geometry"]["coordinates"].clone())?;
        let viewport = MapViewport::for_route(&coords)?;
        let (png, json) = viewport.cache_paths(&dir)?;
        let mut image = RgbImage::from_pixel(1200, 420, Rgb([230, 235, 225]));
        // Mock a road crossing the map; this is a test asset, never a production fallback.
        for x in 0..1200 {
            image.put_pixel(x, 210, Rgb([255, 255, 255]));
        }
        fs::write(png.with_extension("gnis-v1.json"), "[]")?;
        image.save(png)?;
        fs::write(json, serde_json::to_vec(&viewport)?)?;
    }
    Ok(())
}
