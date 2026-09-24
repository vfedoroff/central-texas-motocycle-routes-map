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
                features_to_add = []
                if data.get('type') == 'FeatureCollection' and 'features' in data:
                    features_to_add = data['features']
                elif data.get('type') == 'Feature':
                    features_to_add = [data]

                for feat in features_to_add:
                    feat_props = feat.setdefault('properties', {})
                    feat_props['id'] = item['id']
                    for k in ['category', 'color', 'distance_mi', 'route', 'via', 'waypoints']:
                        if k in item and k not in feat_props:
                            feat_props[k] = item[k]
                    # Round coordinate precision to 5 decimals (~1.1m precision)
                    def round_coords(coords):
                        if isinstance(coords[0], (int, float)):
                            return [round(coords[0], 5), round(coords[1], 5)]
                        return [round_coords(c) for c in coords]
                    if 'geometry' in feat and 'coordinates' in feat['geometry']:
                        feat['geometry']['coordinates'] = round_coords(feat['geometry']['coordinates'])
                    bundled_features.append(feat)
        else:
            print(f"Warning: GeoJSON file missing {gj_file}")

    bundled_geojson = {
        "type": "FeatureCollection",
        "features": bundled_features
    }

    geojson_out_path = os.path.join(ROOT_DIR, 'texas-routes-v2.geojson')
    with open(geojson_out_path, 'w', encoding='utf-8') as f:
        # Minified JSON export for fast CDN transfer & low latency
        json.dump(bundled_geojson, f, separators=(',', ':'))
    print(f"✓ Bundled {len(bundled_features)} features into {geojson_out_path} (minified, {os.path.getsize(geojson_out_path) / 1024:.1f} KB)")

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

    # 3. Generate sitemap.xml
    from datetime import datetime, timezone
    today = datetime.now(timezone.utc).strftime('%Y-%m-%d')
    base_url = 'https://central-texas-routes-map.netlify.app'

    sitemap_lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
        '  <url>',
        f'    <loc>{base_url}/</loc>',
        f'    <lastmod>{today}</lastmod>',
        '    <changefreq>weekly</changefreq>',
        '    <priority>1.0</priority>',
        '  </url>',
        '  <url>',
        f'    <loc>{base_url}/texas-routes-v2.gpx</loc>',
        f'    <lastmod>{today}</lastmod>',
        '    <changefreq>monthly</changefreq>',
        '    <priority>0.8</priority>',
        '  </url>',
        '  <url>',
        f'    <loc>{base_url}/texas-routes-v2.geojson</loc>',
        f'    <lastmod>{today}</lastmod>',
        '    <changefreq>monthly</changefreq>',
        '    <priority>0.7</priority>',
        '  </url>',
    ]

    for item in routes_manifest:
        gpx_rel = item.get('gpx')
        if gpx_rel:
            sitemap_lines.extend([
                '  <url>',
                f'    <loc>{base_url}/{gpx_rel}</loc>',
                f'    <lastmod>{today}</lastmod>',
                '    <changefreq>monthly</changefreq>',
                '    <priority>0.6</priority>',
                '  </url>'
            ])

    sitemap_lines.append('</urlset>\n')
    sitemap_out_path = os.path.join(ROOT_DIR, 'sitemap.xml')
    with open(sitemap_out_path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(sitemap_lines))
    print(f"✓ Generated sitemap with {3 + len(routes_manifest)} URLs at {sitemap_out_path}")

    # 4. Inject Schema.org JSON-LD & Pre-rendered Crawlable Semantic HTML into index.html
    index_html_path = os.path.join(ROOT_DIR, 'index.html')
    if os.path.exists(index_html_path):
        import re
        with open(index_html_path, 'r', encoding='utf-8') as f:
            html = f.read()

        schema_data = {
            '@context': 'https://schema.org',
            '@graph': [
                {
                    '@type': 'WebSite',
                    '@id': f'{base_url}/#website',
                    'url': f'{base_url}/',
                    'name': 'Central Texas Motorcycle Routes Map',
                    'description': f'Interactive map of {len(routes_manifest)} curated, road-snapped Central Texas motorcycle routes with free GPX downloads for Garmin, OSMAnd, and Beeline.',
                    'inLanguage': 'en-US',
                    'publisher': {
                        '@type': 'Person',
                        'name': 'Vadym Fedorov'
                    }
                },
                {
                    '@type': 'ItemList',
                    '@id': f'{base_url}/#routes',
                    'name': 'Central Texas Motorcycle Routes Catalog',
                    'description': f'{len(routes_manifest)} curated scenic motorcycle loops across the Texas Hill Country and Central Texas.',
                    'numberOfItems': len(routes_manifest),
                    'itemListElement': [
                        {
                            '@type': 'ListItem',
                            'position': i + 1,
                            'item': {
                                '@type': 'TouristTrip',
                                '@id': f'{base_url}/#{r["id"]}',
                                'name': r['route'],
                                'description': f'{r["distance_mi"]} mi motorcycle route via {r.get("via", "")}. Category: {r.get("category", "")}. Waypoints: {", ".join(r.get("waypoints", []))}.',
                                'touristType': 'Motorcyclist',
                                'url': f'{base_url}/#{r["id"]}',
                                'hasMap': f'{base_url}/#{r["id"]}',
                                'downloadUrl': f'{base_url}/{r.get("gpx", "")}',
                                'itinerary': {
                                    '@type': 'ItemList',
                                    'numberOfItems': len(r.get('waypoints', [])),
                                    'itemListElement': [
                                        {'@type': 'ListItem', 'position': j + 1, 'name': wp}
                                        for j, wp in enumerate(r.get('waypoints', []))
                                    ]
                                }
                            }
                        }
                        for i, r in enumerate(routes_manifest)
                    ]
                }
            ]
        }
        schema_json = json.dumps(schema_data, indent=2)
        schema_block = f'<!-- SEO_SCHEMA_START -->\n  <script type="application/ld+json">\n{schema_json}\n  </script>\n  <!-- SEO_SCHEMA_END -->'

        html = re.sub(
            r'<!-- SEO_SCHEMA_START -->.*?<!-- SEO_SCHEMA_END -->',
            schema_block,
            html,
            flags=re.DOTALL
        )

        cards = []
        for r in routes_manifest:
            wp_spans = ''.join(f'<span class="waypoint-tag">{wp}</span>' for wp in r.get('waypoints', []))
            card = f'''      <article class="route-card" id="card-{r['id']}">
        <div class="card-top">
          <input type="checkbox" class="route-toggle-checkbox" title="Toggle visibility on map" data-id="{r['id']}" checked/>
          <div class="card-header-info">
            <div class="card-title-row">
              <h3 class="card-title">{r['route']}</h3>
              <span class="card-distance">{r['distance_mi']} mi</span>
            </div>
            <div style="display:flex;align-items:center;gap:6px;margin-top:3px;flex-wrap:wrap;">
              <span class="badge-pill" style="background:{r['color']}">{r['category']}</span>
            </div>
          </div>
        </div>
        <div class="card-via">{r.get('via', '')}</div>
        <div class="waypoints-tags">{wp_spans}</div>
        <div class="card-actions">
          <button class="card-btn zoom-btn" data-id="{r['id']}">🔍 View Route</button>
          <a class="card-btn gpx-download" href="{r['gpx']}" download data-stop-propagation="true">⬇ GPX</a>
        </div>
      </article>'''
            cards.append(card)

        cards_html = '\n'.join(cards)
        routes_block = f'<!-- SEO_ROUTES_START -->\n{cards_html}\n      <!-- SEO_ROUTES_END -->'
        html = re.sub(
            r'<!-- SEO_ROUTES_START -->.*?<!-- SEO_ROUTES_END -->',
            routes_block,
            html,
            flags=re.DOTALL
        )

        total_miles = round(sum(r.get('distance_mi', 0) for r in routes_manifest))
        noscript_items = []
        for r in routes_manifest:
            wp_str = ', '.join(r.get('waypoints', []))
            item = f'''          <div style="border-bottom: 1px solid #e2e8f0; padding-bottom: 10px; margin-bottom: 10px;">
            <h3 style="font-size: 14px; margin-bottom: 3px;"><a href="#{r['id']}" style="color: #0f172a; text-decoration: none; font-weight: 700;">{r['route']}</a> ({r['distance_mi']} mi)</h3>
            <p style="font-size: 12px; color: #475569; margin-bottom: 3px;"><strong>Category:</strong> {r['category']} | <strong>Via:</strong> {r.get('via', '')}</p>
            <p style="font-size: 12px; color: #64748b; margin-bottom: 4px;"><strong>Waypoints:</strong> {wp_str}</p>
            <a href="{r['gpx']}" download style="font-size: 12px; color: #2563eb; font-weight: 600;">⬇ Download GPX Track</a>
          </div>'''
            noscript_items.append(item)

        noscript_content = '\n'.join(noscript_items)
        noscript_block = f'''<!-- SEO_NOSCRIPT_START -->
    <noscript>
      <section class="noscript-catalog" style="padding: 16px; background: #ffffff; margin: 12px; border-radius: 8px; font-size: 13px; line-height: 1.5; color: #1e293b;">
        <h2 style="font-size: 16px; margin-bottom: 8px; color: #0f172a;">Central Texas Motorcycle Routes Catalog ({len(routes_manifest)} Loops • {total_miles} Miles)</h2>
        <p style="margin-bottom: 12px;">Curated collection of 26 road-snapped motorcycle day loops and scenic corridors through the Texas Hill Country, Highland Lakes, and post-oak savannahs. Designed for street motorcycles, with 0 dirt spurs or parking lot incursions.</p>
        <p style="margin-bottom: 14px;">
          <a href="texas-routes-v2.gpx" download style="color: #2563eb; font-weight: 600;">⬇ Download All {len(routes_manifest)} Routes (Monolithic GPX Bundle)</a> | 
          <a href="texas-routes-v2.geojson" download style="color: #2563eb; font-weight: 600;">⬇ Download All Routes (GeoJSON)</a>
        </p>
        <div style="display: flex; flex-direction: column;">
{noscript_content}
        </div>
      </section>
    </noscript>
    <!-- SEO_NOSCRIPT_END -->'''

        html = re.sub(
            r'<!-- SEO_NOSCRIPT_START -->.*?<!-- SEO_NOSCRIPT_END -->',
            noscript_block,
            html,
            flags=re.DOTALL
        )

        with open(index_html_path, 'w', encoding='utf-8') as f:
            f.write(html)
        print(f"✓ Injected SEO Schema.org JSON-LD and crawlable semantic HTML into {index_html_path}")

if __name__ == '__main__':
    bundle()
