use serde_json::{Value, json};

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Generate GPX 1.1 XML string for a route.
/// GeoJSON coordinates are [lon, lat], whereas GPX <trkpt> attributes are lat and lon.
pub fn generate_gpx(title: &str, coords: &[[f64; 2]]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<gpx version=\"1.1\" creator=\"Ride Atlas\" xmlns=\"http://www.topografix.com/GPX/1/1\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.topografix.com/GPX/1/1 http://www.topografix.com/GPX/1/1/gpx.xsd\">\n");
    out.push_str("  <trk>\n");
    out.push_str(&format!("    <name>{}</name>\n", xml_escape(title)));
    out.push_str("    <trkseg>\n");
    for pt in coords {
        let lon = pt[0];
        let lat = pt[1];
        out.push_str(&format!("      <trkpt lat=\"{}\" lon=\"{}\"/>\n", lat, lon));
    }
    out.push_str("    </trkseg>\n");
    out.push_str("  </trk>\n");
    out.push_str("</gpx>\n");
    out
}

/// Generate standalone route GeoJSON Feature for downloads.
pub fn generate_route_geojson(coords: &[[f64; 2]]) -> Value {
    json!({
        "type": "Feature",
        "properties": {},
        "geometry": {
            "type": "LineString",
            "coordinates": coords
        }
    })
}
