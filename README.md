# Central Texas Motorcycle Routes Map

Interactive map of Central Texas motorcycle routes, road-snapped to real roads.

**Live map:** https://central-texas-routes-map.netlify.app/

| File / Directory | Description |
|------------------|-------------|
| `index.html` | Dynamic interactive web map (Leaflet + OpenStreetMap). Asynchronously loads `routes.json` and GeoJSON route files. |
| `routes.json` | Central route manifest listing all active routes, categories, colors, waypoints, and file paths. |
| `routes/` | Directory containing individual modular GeoJSON route files (e.g. `routes/the-three-twisted-sisters-circuit.geojson`). |
| `gpx/` | Directory containing individual modular GPX track files (e.g. `gpx/the-three-twisted-sisters-circuit.gpx`) for GPS devices. |
| `scripts/bundle.py` | Python script to bundle modular route files into monolithic GeoJSON and GPX files for backward compatibility. |
| `texas-routes-v2.geojson` | Monolithic bundled GeoJSON (26 systematic features, road-snapped). |
| `texas-routes-v2.gpx` | Monolithic bundled GPX (26 tracks, for GPS devices). |

## How to Add a New Route

1. Add your GeoJSON route file into `routes/your-new-route.geojson`.
2. Add your GPX track file into `gpx/your-new-route.gpx`.
3. Add a new entry into `routes.json`:
   ```json
   {
     "id": "your-new-route",
     "route": "Your New Route Name",
     "category": "Twisties & Canyons",
     "color": "#e74c3c",
     "distance_mi": 45.0,
     "waypoints": ["Start Town", "Via Spot", "End Town"],
     "via": "FM 123 -> RM 456",
     "geojson": "routes/your-new-route.geojson",
     "gpx": "gpx/your-new-route.gpx"
   }
   ```
4. Run `python3 scripts/bundle.py` to update the bundled `texas-routes-v2.geojson` and `texas-routes-v2.gpx` files.

## Systematic Routes (26 Categorized Layers)

### 🔴 Category 1: Iconic Hill Country Twisties & Canyons
1. **The Three Twisted Sisters Circuit** (`159.4 mi`) — FM 337 / FM 335 / FM 336 (Leakey → Camp Wood → Vanderpool → Medina). Texas's #1 benchmark motorcycle ride.
2. **Willow City Loop & Enchanted Rock Run** (`60.5 mi`) — TX 16 → RM 1323 → Willow City Loop → FM 965 (Fredericksburg → Willow City → Enchanted Rock).
3. **Devil's Backbone Scenic Ridge** (`38.4 mi`) — RM 12 → RM 32 → US 281 (Wimberley → Fischer → Blanco). Legendary limestone ridge ride with panoramic canyon views.
4. **Balcones Escarpment Twisties** (`29.1 mi`) — FM 1431 → Cow Creek Rd (FM 1174) → RM 1869 (Marble Falls → Liberty Hill). Technical twisties through Balcones Canyonlands.
5. **Lime Creek Road & Lake Travis Loop** (`50.5 mi`) — FM 2769 → Lime Creek Rd → Volente (Cedar Park → Volente). Famous tight twisty corridor hugging Lake Travis bluffs.
6. **Austin to Llano - Hill Country Blast** (`179.5 mi`) — RM 620 South → TX 71 West → Llano → TX 29 East → RM 1431 → Park Road 4 (Inks Lake & Longhorn Cavern) → US 281 North → Burnet → RM 1174 South (Cow Creek Rd) → RM 1431 East → Lime Creek Rd → Volente → Bullick Hollow Rd (Austin → Spicewood → Llano → Inks Lake → Burnet → Balcones Canyonlands → Volente). Epic 179-mile premier twisty circuit stringing together Texas's most celebrated technical roads: Lime Creek Road, Cow Creek canyon, and Park Road 4.

### 🔵 Category 2: Highland Lakes & River Valley Runs
7. **Highland Lakes Circuit** (`60.4 mi`) — TX 29 → RM 2341 → Park Road 4 (Burnet → Lake Buchanan → Inks Lake → Marble Falls). Past Lake Buchanan & Longhorn Cavern.
8. **Kingsland & Lake LBJ Loop** (`56.6 mi`) — Park Road 4 → RM 2900 → RM 2233 → US 71 → RM 2147. Lakeside circuit around Lake LBJ & Packsaddle Mountain.
9. **Guadalupe River Road & Hunt Sweep** (`80.1 mi`) — TX 39 → FM 1340 (Kerrville → Ingram → Hunt → North Fork Guadalupe River). Scenic shaded river ride.
10. **Blanco River Valley & FM 165 Loop** (`84.6 mi`) — FM 165 → Henly → Wimberley → RM 2325 (Blanco → Wimberley). Winding limestone river valley run.
11. **Llano River & Castell Run** (`44.3 mi`) — FM 152 → Castell → FM 2323 (Llano → Castell → Mason). South bank run along the Llano River into granite country.
12. **Saturday Dawn Lake Travis Loop** (`98.8 mi`) — RM 1431 West → US 281 South → East RM 962 → Hamilton Pool Rd → TX 71 East → RM 620 North → Anderson Mill Rd (Cedar Park → Lago Vista → Marble Falls → Round Mountain → Cypress Mill → Bee Cave). Complete ~99-mile Lake Travis dawn circuit circling the entire lake basin, crossing Mansfield Dam and sweeping through Balcones Canyonlands.
13. **Saturday Dawn Lake Granger Loop** (`105.2 mi`) — Ronald Reagan Blvd North → FM 3405 → RM 2338 South → FM 971 East → Granger → FM 1331 (Granger Lake Dam) → TX 95 South → Taylor → FM 1660 → Chandler Rd → University Blvd / RM 1431 West (Cedar Park → Georgetown → Weir → Granger → Taylor → Hutto → Round Rock). Scenic eastern dawn circuit sweeping north along Ronald Reagan Blvd, riding through historic Georgetown, circling Granger Lake and its dam, and returning through Taylor and Hutto.

### 🟣 Category 3: Historic German Towns & Craft Beverage Runs
14. **Sisterdale, Kendalia & Luckenbach Run** (`55.7 mi`) — FM 1376 → FM 473 (Boerne → Sisterdale → Kendalia → Luckenbach). Iconic ride connecting historic dancehalls & rivers.
15. **Pedernales Wine Country Sweep** (`83.8 mi`) — FM 2093 Tivydale → US 290 Wine Road → Stonewall (Fredericksburg → Johnson City). Peach orchards & Pedernales vineyards.
16. **German Heritage Trail** (`64.1 mi`) — Old San Antonio Rd → FM 473 (Fredericksburg → Sisterdale → Blanco). Pioneer settlement corridor.
17. **Fitzhugh & Hamilton Pool Craft Beverage Run** (`24.2 mi`) — Hamilton Pool Rd → Fitzhugh Rd (Bee Cave → Dripping Springs). Winding ridge & artisan distillery corridor.
18. **Leander to Luckenbach Loop** (`219.0 mi`) — Nameless Rd → RM 1431 West → US 281 South → RM 1323 → Willow City Loop → TX 16 → Fredericksburg → RR 1376 → Luckenbach → RR 1888 → Blanco → US 281 → CR 301 → Shovel Mountain Rd → TX 71 → Marble Falls → RM 1431 → Cow Creek Rd (FM 1174) → RM 1869 → Liberty Hill (Leander → Marble Falls → Willow City → Fredericksburg → Luckenbach → Blanco → Johnson City → Liberty Hill). Comprehensive 219-mile grand Hill Country day loop linking iconic German settlements, dancehalls, technical switchback canyons, and scenic river crossings.

### 🟢 Category 4: Post Oak Savannah & Prairie Sweeps
19. **North Gabriel River Sweep** (`22.1 mi`) — TX 29 @ CR 200 → CR 200 → CR 201 → CR 214 → RM 243 → CR 210 → US 183. Winding rural river road starting at TX 29 and CR 200.
20. **Oakalla & Berry Creek Valley Run** (`35.5 mi`) — US 183 → FM 963 North to Oakalla → FM 963 West through Berry Creek Valley → Burnet. Sweeping curves passing directly through Oakalla and the Lampasas River.
21. **Lost Pines & Bastrop State Park Loop** (`55.9 mi`) — FM 1441 → Park Road 1 → Smithville (Bastrop → Buescher State Park). Loblolly pine forest ride.
22. **Round Top & Fayetteville Country Lane** (`38.8 mi`) — FM 1291 → Round Top → FM 2503 (Fayetteville → Round Top → Shelby). Rolling antique prairie country loop.
23. **Mason Mountain Ridge Run** (`74.6 mi`) — TX 29 West → Art Road → US 87 (Mason → Art → Streeter). Western Hill Country red-granite ridge run.
24. **Lake Victor & Burnet Backcountry Sweep** (`27.9 mi`) — RM 2340 → Lake Victor → Watson (Burnet → Lake Victor). Rural ranchland sweep through Burnet County.
25. **Bertram, RM 1174 & RM 963 Loop** (`54.6 mi`) — TX 29 West → Bertram → RM 1174 North → RM 963 East → US 183 South. Scenic full loop starting and ending at the US 183 / TX 29 crossing via Bertram, RM 1174 ranchland sweep, RM 963, and US 183.
26. **Briggs, Oakalla & RM 963 Loop** (`23.1 mi`) — US 183 → Briggs → FM 2657 North → Oakalla → RM 963 West → Watson → US 183 South. Scenic short loop starting at US 183 & CR 218 in Briggs, climbing north to Oakalla, sweeping west across Berry Creek Valley on RM 963, and returning south on US 183.

## Notes

- Routes are snapped to real roads via OSRM. Where the source trace had gaps, the rebuild bridges them along plausible roads (not the original trace).
- The "Red (Burnet County)" dirt-road trace (CR 110/CR 113, classified as dirt by Burnet County) was removed — not suitable for street motorcycles.
- Basemap: OpenStreetMap.

## Deploy

The live site is deployed from `index.html` via Netlify Drop to the `central-texas-routes-map` project.

## License

This project is licensed under the [MIT License](LICENSE) — Copyright (c) 2026 Vadym Fedorov.

