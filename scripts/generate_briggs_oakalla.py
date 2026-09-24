#!/usr/bin/env python3
import urllib.request
import json
import os
import xml.sax.saxutils as saxutils

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.abspath(os.path.join(SCRIPT_DIR, '..'))

route_id = "briggs-oakalla-rm-963-loop"
route_name = "Briggs, Oakalla & RM 963 Loop"
category = "Savannah & Prairie Sweeps"
color = "#27ae60"
waypoints = [
    "US 183 / CR 218",
    "Briggs",
    "FM 2657",
    "Oakalla",
    "RM 963",
    "Watson",
    "US 183"
]
via = "US 183 -> Briggs -> FM 2657 North -> Oakalla -> RM 963 West -> Watson -> US 183 South"

# Precise waypoints along the loop:
# 1. US 183 & CR 218 (just south of Briggs): (-97.9255, 30.8845)
# 2. Oakalla (FM 2657 & RM 963): (-97.9253, 30.9863)
# 3. Watson (RM 963 & US 183): (-98.01449, 30.934326)
# 4. Return to start (US 183 & CR 218): (-97.9255, 30.8845)
coords = [
    (-97.9255, 30.8845),
    (-97.9253, 30.9863),
    (-98.01449, 30.934326),
    (-97.9255, 30.8845)
]

coords_str = ";".join(f"{c[0]},{c[1]}" for c in coords)
url = f"http://router.project-osrm.org/route/v1/driving/{coords_str}?overview=full&geometries=geojson"
req = urllib.request.Request(url, headers={"User-Agent": "Antigravity/1.0"})
with urllib.request.urlopen(req) as r:
    data = json.loads(r.read().decode())

route = data["routes"][0]
geometry = route["geometry"]
dist_mi = round(route["distance"] / 1609.344, 1)

print(f"Route distance: {dist_mi} miles, {len(geometry['coordinates'])} coordinates")

# 1. Create GeoJSON
geojson_obj = {
    "type": "FeatureCollection",
    "features": [
        {
            "type": "Feature",
            "properties": {
                "route": route_name,
                "category": category,
                "color": color,
                "waypoints": waypoints,
                "via": via,
                "distance_mi": dist_mi,
                "source": "road-snapped"
            },
            "geometry": geometry
        }
    ]
}

geojson_path = os.path.join(ROOT_DIR, f"routes/{route_id}.geojson")
with open(geojson_path, "w", encoding="utf-8") as f:
    json.dump(geojson_obj, f, indent=2)
print(f"Created {geojson_path}")

# 2. Create GPX
route_name_xml = saxutils.escape(route_name)
category_xml = saxutils.escape(category)
via_xml = saxutils.escape(via)
gpx_lines = [
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
    "<gpx version=\"1.1\" creator=\"Antigravity Route Generator\" xmlns=\"http://www.topografix.com/GPX/1/1\">",
    "  <trk>",
    f"    <name>{route_name_xml}</name>",
    f"    <desc>{category_xml} ({dist_mi} mi) - {via_xml}</desc>",
    "    <trkseg>"
]

for coord in geometry["coordinates"]:
    lon, lat = coord[0], coord[1]
    gpx_lines.append(f'      <trkpt lat="{lat}" lon="{lon}" />')

gpx_lines.extend([
    "    </trkseg>",
    "  </trk>",
    "</gpx>"
])

gpx_path = os.path.join(ROOT_DIR, f"gpx/{route_id}.gpx")
with open(gpx_path, "w", encoding="utf-8") as f:
    f.write("\n".join(gpx_lines) + "\n")
print(f"Created {gpx_path}")
