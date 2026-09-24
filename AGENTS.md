# Agent Guide: Adding & Managing Motorcycle Routes

This document provides instructions for AI agents and developers on how to add, edit, validate, and bundle routes in this repository.

---

## 1. Project Architecture

The repository uses a **modular route architecture**:

```
.
├── routes.json                 # Central route manifest (single source of truth)
├── routes/                     # Individual modular GeoJSON route files
│   ├── the-three-twisted-sisters-circuit.geojson
│   └── ...
├── gpx/                        # Individual modular GPX 1.1 track files
│   ├── the-three-twisted-sisters-circuit.gpx
│   └── ...
├── scripts/
│   └── bundle.py               # Compiles routes/ & gpx/ into monolithic deliverables
├── texas-routes-v2.geojson     # Generated: All routes bundled into one GeoJSON
├── texas-routes-v2.gpx         # Generated: All tracks bundled into one GPX
├── index.html                  # Interactive Leaflet web map viewer
└── README.md                   # Public documentation & catalog
```

---

## 2. Route Standards & Guidelines

When curating or adding a route:
1. **Paved Surfaces Only**: Must be suitable for street motorcycles. Exclude dirt/unpaved county roads (e.g. Burnet County unpaved roads).
2. **Loop or Scenic Corridor**: Prefer circular day-loops (15–180 miles) or scenic riverside/ridge corridors.
3. **No Dead-End Spurs**: Prune accidental out-and-back ranch road spurs or junction overshoot detours.
4. **Snapped to Roads**: Road geometries must be snapped to actual highway and road centerlines (via OSRM, GraphHopper, or real GPS traces).

---

## 3. Step-by-Step: How to Add a New Route

### Step 1: Choose Route ID and Category

Generate a unique `kebab-case` ID (e.g., `bertram-rm-1174-rm-963-loop`).

Assign the route to one of the 4 defined categories and use its designated hex color:

| Category | Color | Description / Examples |
|---|---|---|
| **Twisties & Canyons** | `#e74c3c` (Red) | High-elevation changes, technical switchbacks, canyons (e.g., Twisted Sisters, Lime Creek) |
| **Lakes & Rivers** | `#2980b9` (Blue) | Highland Lakes, river crossings, dam sweeps (e.g., Lake LBJ, Guadalupe River, Llano River) |
| **German Towns & Culture** | `#8e44ad` (Purple) | Historic settlement trails, wineries, dancehalls (e.g., Fredericksburg, Luckenbach, Sisterdale) |
| **Savannah & Prairie Sweeps** | `#27ae60` (Green) | Rolling country lanes, post-oak pastures, ranch roads (e.g., Round Top, Bastrop, Oakalla, Bertram) |

---

### Step 2: Create GeoJSON File (`routes/<id>.geojson`)

Create `routes/<id>.geojson` as a standard GeoJSON Feature or FeatureCollection.

**Coordinate convention**: GeoJSON uses `[longitude, latitude]` order.

```json
{
  "type": "Feature",
  "properties": {
    "name": "Bertram, RM 1174 & RM 963 Loop",
    "category": "Savannah & Prairie Sweeps",
    "color": "#27ae60",
    "distance_mi": 54.6
  },
  "geometry": {
    "type": "LineString",
    "coordinates": [
      [-97.8765, 30.7423],
      [-97.8770, 30.7430]
    ]
  }
}
```

---

### Step 3: Create GPX File (`gpx/<id>.gpx`)

Create `gpx/<id>.gpx` formatted in GPX 1.1 with track points (`<trkpt>`).

**Coordinate convention**: GPX uses attributes `lat="..." lon="..."`.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Antigravity Route Curation" xmlns="http://www.topografix.com/GPX/1/1">
  <trk>
    <name>Bertram, RM 1174 & RM 963 Loop</name>
    <trkseg>
      <trkpt lat="30.7423" lon="-97.8765"></trkpt>
      <trkpt lat="30.7430" lon="-97.8770"></trkpt>
    </trkseg>
  </trk>
</gpx>
```

---

### Step 4: Register in `routes.json`

Append the route entry to the JSON array in [`routes.json`](routes.json):

```json
{
  "id": "bertram-rm-1174-rm-963-loop",
  "route": "Bertram, RM 1174 & RM 963 Loop",
  "category": "Savannah & Prairie Sweeps",
  "color": "#27ae60",
  "distance_mi": 54.6,
  "waypoints": [
    "183 / TX 29 Crossing",
    "Bertram",
    "RM 1174",
    "RM 963",
    "US 183"
  ],
  "via": "TX 29 West -> Bertram -> RM 1174 North -> RM 963 East -> US 183 South",
  "geojson": "routes/bertram-rm-1174-rm-963-loop.geojson",
  "gpx": "gpx/bertram-rm-1174-rm-963-loop.gpx"
}
```

#### Field Schema:
- `id` (string, required): Unique `kebab-case` identifier matching filenames.
- `route` (string, required): Full descriptive title of the route.
- `category` (string, required): One of the 4 defined categories above.
- `color` (string, required): Hex color matching category standard.
- `distance_mi` (number, required): Mileage rounded to 1 decimal place.
- `waypoints` (array of strings, required): Key towns, intersections, or geographic landmarks along the route. Used by real-time search.
- `via` (string, required): Ordered list of highways and ranch roads (e.g. `TX 29 -> RM 1174 -> RM 963`).
- `geojson` (string, required): Relative path to GeoJSON file in `routes/`.
- `gpx` (string, required): Relative path to GPX file in `gpx/`.

---

### Step 5: Rebuild Monolithic Files

Run the bundler script:

```bash
python3 scripts/bundle.py
```

This updates:
- `texas-routes-v2.geojson`
- `texas-routes-v2.gpx`

---

### Step 6: Update Documentation

Add the new route to the numbered catalog in [`README.md`](README.md) under its respective category section:
```markdown
21. **Bertram, RM 1174 & RM 963 Loop** (`54.6 mi`) — TX 29 West -> Bertram -> RM 1174 North -> RM 963 East -> US 183 South. Scenic full loop...
```

---

## 4. Verification & Testing

1. **Verify Files Exist & JSON Syntax**:
   ```bash
   python3 -c "import json; routes = json.load(open('routes.json')); print(f'{len(routes)} valid routes')"
   ```
2. **Launch Local Server**:
   ```bash
   python3 -m http.server 8000
   ```
3. **Check in Browser (`http://localhost:8000`)**:
   - The new route appears in the sidebar list.
   - Dynamic route count badge and category pill count increment automatically.
   - Searching by town, road name, or title highlights the new route card.
   - Clicking the card pans/zooms to the route bounding box and opens its popup.
   - Direct GPX download button works.
