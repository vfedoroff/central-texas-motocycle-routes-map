# Ride Atlas Agent Guide

## Architecture

Authored catalog records and geometry live in `content/`; site configuration lives in `config/`. The Rust workspace contains catalog-model, catalog-build, catalog-ui and xtask. The Leptos WebAssembly app uses Leaflet. Browser tests live in `tests/browser/`.

## Generated output

Never manually edit or commit `dist/`, `.build/`, `target/` or node_modules. Build through xtask.

## Development

Use the pinned toolchain in mise.toml and rust-toolchain.toml. Run `mise install`, `mise run setup`, and `npm ci` for setup.

- `cargo xtask check`: formatting, Clippy and workspace tests.
- `cargo xtask validate`: catalog schema, references and geometry validation.
- `cargo xtask build`: production static site with optimized WebAssembly.
- `cargo xtask build --development`: local development build with analytics disabled.
- `npm run preview`: local server at http://127.0.0.1:8000.
- `npm run test:browser`: desktop and mobile tests.

## Content

Use stable kebab-case IDs. A route has metadata in content/routes and GeoJSON in content/geometry/routes. Coordinates use [longitude, latitude]; closed loops use identical first and last coordinates. Maintain references when editing or deleting objects. Cite factual sources and do not invent riding claims. Place records belong in content/places. Keep generated downloads out of Git.

### Route descriptions and ride notes

- `summary`: one short sentence identifying the route for the catalog card. Keep detailed opinions in ride notes.
- `body_markdown`: useful route context, main roads, and stops. Do not repeat the itinerary, distance, or ride notes just to fill this field; leave it empty when there is nothing to add.
- `author_note.text`: the rider’s experience of traffic, pavement, pace, scenery, and stop preferences. Preserve their meaning and voice. Do not invent visits, recommendations, or riding experiences.
- `stops[].note`: practical information specific to that stop on this ride.
- Review these sections together when editing a route. Move personal observations into ride notes and remove duplicated passages from the description without losing details.

### Route colors

Choose colors to distinguish individual routes, independently of their categories. A different hex value alone is not enough: nearby shades can still look identical on the map.

- Inspect the colors in `content/routes/` and the new route’s geometry. Prioritize routes that share roads, cross, or appear together at the same map zoom.
- Use a qualitative palette with distinct hues, such as a ColorBrewer palette with the colorblind-safe filter enabled. Do not use a sequential gradient for route identities or automatically reuse the category color.
- Prefer a hue absent from nearby routes. If those routes are green, choose a clearly different hue rather than another green or teal. Also compare lightness and saturation; avoid relying only on red versus green.
- Use an unused color when practical. Reuse a color only for geographically separated routes that are unlikely to appear together at riding zoom levels. Do not recolor unrelated routes without a reason.
- Check the candidate against the actual basemap at overview and riding zoom levels, with neighboring routes visible. It must stand out from terrain, water, roads, and nearby route lines. Avoid pale yellow or other low-contrast strokes on the light basemap.
- Review common color-vision deficiencies with a simulator when available. A palette’s accessibility label does not guarantee that every combination works on this map. Keep route names and selection feedback usable because color alone cannot identify coincident lines.
- Validate the catalog and rebuild the development site after changing a color. Report the chosen hex value and which nearby colors it was selected to contrast with.

References: [ColorBrewer map color guidance](https://colorbrewer2.org/) and [Esri: Cartographic creations—Style thematic data](https://learn.arcgis.com/en/projects/cartographic-creations-style-thematic-data/).

## Git and release

Shared route links must use `/routes/{id}/` so social crawlers can read the static page without JavaScript. Keep Open Graph and X card metadata specific to the ride, with absolute image URLs. Generate preview images from route geometry during the catalog build; do not edit generated images by hand. Preview images include the ride title, distance, and Ride Atlas branding. Shared pages open the selected ride on the interactive map for visitors; retain a readable page and preview image when JavaScript is unavailable. Use one Share ride action with native sharing and a clipboard fallback.

Create signed Conventional Commits when requested. Push only when authorized. Internal docs, QA records and plans remain local and ignored. Netlify builds production output from main using netlify.toml.
