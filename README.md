# Central Texas Motorcycle Routes Map

Interactive map of Central Texas motorcycle routes, road-snapped to real roads.

**Live map:** https://central-texas-routes-map.netlify.app/

## Files

| File | Description |
|------|-------------|
| `index.html` | Standalone interactive map (Leaflet + OpenStreetMap). Deployed to Netlify. |
| `texas-routes-v2.geojson` | All routes as GeoJSON (24 features, road-snapped). |
| `texas-routes-v2.gpx` | All routes as GPX tracks (23 tracks, for GPS devices). |

## Routes (24 layers)

**Named routes:**
- Harper – Austin (via Johnson City)
- Comfort – Fayetteville (via Manor)
- Kendalia – Gause (via Lexington)
- Mason – Lexington (via Llano)
- Barksdale – Willow City (via Ingram)
- Lometa – Giddings (via Leander)
- Fredericksburg – Lincoln (via Manor)
- Kendall County link
- TX 29 – US 183 (via CR 201 / CR 214 / CR 210)
- Willow City Loop (Fredericksburg → TX 16 → RM 1323 → 13-mi loop, 34.9 mi)
- Marble Falls – Liberty Hill (FM 1431 / 1174 / 1869, 29.1 mi)
- RM 2340 (Burnet County, 7.6 mi)
- RM 963 (US 183 → RM 963, 25.8 mi)
- Ride 1: Burnet → Marble Falls (60.4 mi)
- Ride 2: Marble Falls Loop (57.1 mi)

**County fragments (Magenta):** Bastrop, Blanco, Burnet, Fayette, Gillespie, Kendall, Llano, Mason

**Red fragments:** Hays County

## Notes

- Routes are snapped to real roads via OSRM. Where the source trace had gaps, the rebuild bridges them along plausible roads (not the original trace).
- The "Red (Burnet County)" dirt-road trace (CR 110/CR 113, classified as dirt by Burnet County) was removed — not suitable for street motorcycles.
- Basemap: OpenStreetMap.

## Deploy

The live site is deployed from `index.html` via Netlify Drop to the `central-texas-routes-map` project.
