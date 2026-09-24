#!/usr/bin/env python3
"""
Bundle script for Central Texas Motorcycle Routes Map.
Reads routes.json and bundles all individual routes/*.geojson and gpx/*.gpx files
into monolithic texas-routes-v2.geojson and texas-routes-v2.gpx for backward compatibility.
"""

import os
import json
import xml.etree.ElementTree as ET

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.abspath(os.path.join(SCRIPT_DIR, '..'))

def bundle():
    manifest_path = os.path.join(ROOT_DIR, 'routes.json')
    if not os.path.exists(manifest_path):
        print(f"Error: {manifest_path} not found.")
        return

    with open(manifest_path, 'r', encoding='utf-8') as f:
        routes_manifest = json.load(f)

    # 1. Bundle GeoJSON
    bundled_features = []
    for item in routes_manifest:
        gj_file = os.path.join(ROOT_DIR, item['geojson'])
        if os.path.exists(gj_file):
            with open(gj_file, 'r', encoding='utf-8') as f:
                data = json.load(f)
                if data.get('type') == 'FeatureCollection' and 'features' in data:
                    bundled_features.extend(data['features'])
                elif data.get('type') == 'Feature':
                    bundled_features.append(data)
        else:
            print(f"Warning: GeoJSON file missing {gj_file}")

    bundled_geojson = {
        "type": "FeatureCollection",
        "features": bundled_features
    }

    geojson_out_path = os.path.join(ROOT_DIR, 'texas-routes-v2.geojson')
    with open(geojson_out_path, 'w', encoding='utf-8') as f:
        json.dump(bundled_geojson, f, indent=2)
    print(f"✓ Bundled {len(bundled_features)} features into {geojson_out_path}")

    # 2. Bundle GPX
    gpx_root = ET.Element('gpx', {
        'version': '1.1',
        'creator': 'Antigravity Route Bundler',
        'xmlns': 'http://www.topografix.com/GPX/1/1'
    })

    ns = {'gpx': 'http://www.topografix.com/GPX/1/1'}
    track_count = 0

    for item in routes_manifest:
        gpx_file = os.path.join(ROOT_DIR, item['gpx'])
        if os.path.exists(gpx_file):
            tree = ET.parse(gpx_file)
            tracks = tree.getroot().findall('gpx:trk', ns)
            if not tracks:
                # Retry finding without namespace
                tracks = tree.getroot().findall('trk')
            for trk in tracks:
                gpx_root.append(trk)
                track_count += 1
        else:
            print(f"Warning: GPX file missing {gpx_file}")

    gpx_tree = ET.ElementTree(gpx_root)
    ET.indent(gpx_tree, space='  ')
    gpx_out_path = os.path.join(ROOT_DIR, 'texas-routes-v2.gpx')
    gpx_tree.write(gpx_out_path, encoding='UTF-8', xml_declaration=True)
    print(f"✓ Bundled {track_count} tracks into {gpx_out_path}")

if __name__ == '__main__':
    bundle()
