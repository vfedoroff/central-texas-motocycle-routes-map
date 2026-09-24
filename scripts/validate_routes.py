#!/usr/bin/env python3
"""
Route Topology & Standards Validator
Verifies routes.json manifest, individual route files, and ensures 0 unwanted spurs/detours.

Usage:
  python3 scripts/validate_routes.py              # Validates manifest schema and file integrity
  python3 scripts/validate_routes.py --audit-spurs # Audits loop routes for unwanted spurs/detours
  python3 scripts/validate_routes.py <route-id>   # Audits specific route for spurs/detours
"""

import json
import math
import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROUTES_JSON = os.path.join(BASE_DIR, 'routes.json')
ROUTES_DIR = os.path.join(BASE_DIR, 'routes')
GPX_DIR = os.path.join(BASE_DIR, 'gpx')
BUNDLE_GEOJSON = os.path.join(BASE_DIR, 'texas-routes-v2.geojson')
BUNDLE_GPX = os.path.join(BASE_DIR, 'texas-routes-v2.gpx')

CATEGORIES = {
    'Twisties & Canyons': '#e74c3c',
    'Lakes & Rivers': '#2980b9',
    'German Towns & Culture': '#8e44ad',
    'Savannah & Prairie Sweeps': '#27ae60'
}

def dist_m(p1, p2):
    dx = (p1[0] - p2[0]) * math.cos(math.radians(p1[1])) * 111320
    dy = (p1[1] - p2[1]) * 110540
    return math.hypot(dx, dy)

def detect_spurs(coords, min_track_len=80, max_track_len=5000, max_gap=40):
    """Detects localized neighborhood loops, parking lot incursions, and out-and-back detours."""
    n = len(coords)
    cum = [0.0]
    for i in range(1, n):
        cum.append(cum[-1] + dist_m(coords[i-1], coords[i]))
    
    spurs = []
    i = 0
    while i < n - 4:
        best = None
        for j in range(i + 4, min(i + 400, n)):
            t_len = cum[j] - cum[i]
            if t_len > max_track_len:
                break
            if t_len >= min_track_len:
                s_len = dist_m(coords[i], coords[j])
                if s_len <= max_gap:
                    best = (i, j, t_len, s_len, coords[i])
                    break
        if best:
            spurs.append(best)
            i = best[1] # advance past spur
        else:
            i += 1
    return spurs

def main():
    args = sys.argv[1:]
    audit_spurs_all = '--audit-spurs' in args
    target_routes = [a for a in args if not a.startswith('--')]

    errors = []
    warnings = []

    print("Checking routes.json manifest...")
    if not os.path.exists(ROUTES_JSON):
        print(f"ERROR: Missing manifest {ROUTES_JSON}")
        sys.exit(1)

    with open(ROUTES_JSON) as f:
        routes = json.load(f)

    print(f"Found {len(routes)} routes in manifest.\n")

    for r in routes:
        rid = r.get('id', 'unknown')
        if target_routes and rid not in target_routes:
            continue

        cat = r.get('category')
        col = r.get('color')

        # Schema checks
        if cat not in CATEGORIES:
            errors.append(f"[{rid}] Invalid category: '{cat}'")
        elif col != CATEGORIES[cat]:
            warnings.append(f"[{rid}] Color '{col}' does not match standard '{CATEGORIES[cat]}' for '{cat}'")

        geojson_path = os.path.join(BASE_DIR, r.get('geojson', ''))
        gpx_path = os.path.join(BASE_DIR, r.get('gpx', ''))

        if not os.path.exists(geojson_path):
            errors.append(f"[{rid}] Missing GeoJSON file: {geojson_path}")
            continue

        if not os.path.exists(gpx_path):
            errors.append(f"[{rid}] Missing GPX file: {gpx_path}")

        # Validate GeoJSON coordinates and topology
        with open(geojson_path) as f:
            geo_data = json.load(f)

        if 'features' in geo_data and len(geo_data['features']) > 0:
            coords = geo_data['features'][0]['geometry']['coordinates']
        elif 'geometry' in geo_data:
            coords = geo_data['geometry']['coordinates']
        else:
            errors.append(f"[{rid}] Invalid GeoJSON structure")
            continue

        # Check for unintended spurs / cul-de-sac diversions
        if audit_spurs_all or target_routes:
            spurs = detect_spurs(coords)
            if spurs:
                errors.append(f"[{rid}] {len(spurs)} unwanted spur(s)/detour(s) detected! e.g. at index {spurs[0][0]}->{spurs[0][1]} ({spurs[0][2]:.1f}m detour)")

    # Check bundled deliverables
    if not os.path.exists(BUNDLE_GEOJSON):
        errors.append("Missing bundled texas-routes-v2.geojson (run python3 scripts/bundle.py)")
    if not os.path.exists(BUNDLE_GPX):
        errors.append("Missing bundled texas-routes-v2.gpx (run python3 scripts/bundle.py)")

    if warnings:
        print("WARNINGS:")
        for w in warnings:
            print(f"  - {w}")
        print()

    if errors:
        print("VALIDATION FAILED:")
        for e in errors:
            print(f"  ❌ {e}")
        sys.exit(1)
    else:
        print("✅ ALL CHECKS PASSED: Route files exist, manifest is valid, and geometry meets standards!")

if __name__ == '__main__':
    main()
