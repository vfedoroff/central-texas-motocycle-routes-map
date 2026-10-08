# Ride Atlas: Central Texas Motorcycle Routes

Browse Central Texas motorcycle routes on an interactive map. Find rides by category, distance, search, or map area; save favorites; share links to individual routes; export GPX and GeoJSON; and get directions to a ride’s starting coordinate.

[Live site](https://central-texas-routes-map.netlify.app)

## Rider features

- Responsive route and place catalog with sorting, filters, and map markers.
- Saved rides persist in the browser. Offline packs cache route details, geometry, and GPX; **map tiles are not included**.
- Outdated or incomplete packs explain their state and offer update and removal actions.
- “Directions to start” navigates to the starting coordinate, not along the entire ride.
- Moving the map leaves list results unchanged until you choose “Search this area.”
- Optional Google Analytics loads only after consent on the configured production host.

## Architecture

A Rust workspace generates a static site with a Leptos/WebAssembly UI and Leaflet map.

```text
content/                     Authored routes, geometry, places, roads and audit inputs
config/                      Site and road-audit settings
crates/catalog-model/        Domain models and schema validation
crates/catalog-roads/        Road topology, alignment and audit evidence
crates/catalog-build/        Static pages, exports and service worker generation
crates/catalog-ui/           Leptos UI and JavaScript browser adapters
crates/route-lint/            Read-only route verification CLI
crates/xtask/                 Build and validation commands
tests/browser/               Desktop and mobile Playwright tests
dist/                        Generated site; never edit or commit
```

## Local development

Pinned versions are in `mise.toml` and `rust-toolchain.toml`: Rust 1.99.0, Node 24.21.0 and Trunk 0.21.14.

```sh
mise trust
mise install
mise run setup
npm ci
mise exec -- cargo xtask build --development
mise exec -- npm run preview
```

Open http://127.0.0.1:8000. Development builds allow unverified route previews and disable analytics.

```sh
cargo xtask check              # Formatting, Clippy and workspace tests
cargo xtask validate           # Authored catalog validation
npm run test:browser           # Desktop and mobile browser tests
cargo xtask build              # Production build
```

If macOS `rust-lld` cannot locate `libLLVM.dylib`, supply the installed toolchain’s library directory through `DYLD_FALLBACK_LIBRARY_PATH` for the build command.

## Route authoring

Edit authored files in `content/` and `config/`. Routes require metadata and canonical road geometry. Verified network paths and audit evidence can be added for optional road audits. Use stable IDs; keep all references consistent. Routes must be paved, follow public road centerlines, avoid parking-lot incursions and unwanted spurs, and cite factual sources. Closed loops must have identical first and last coordinates.

See [AGENTS.md](AGENTS.md), [CONTRIBUTING.md](CONTRIBUTING.md) for the authoring and audit workflows. Do not invent verification evidence.

## Netlify deployment

Netlify publishes `dist/` from the production branch, normally `main`. The build configuration installs the pinned Trunk version and WebAssembly target before running `cargo xtask build`. Node dependencies are installed from `package-lock.json`.

Production builds validate catalog schemas and geometry, generate optimized WebAssembly and static assets, and use production analytics settings. Road-network audits are optional maintenance checks, not deployment prerequisites. To explicitly require road evidence, use `cargo run --locked -p catalog-build -- build --environment production` with the verified network and audit inputs available.

After the production build succeeds, commit the authored sources and build configuration, merge into `main`, and push to the connected Git repository. Confirm the Netlify deployment succeeds before treating the live site as updated. Generated `dist/` stays untracked.

Netlify references: [dependencies](https://docs.netlify.com/build/configure-builds/manage-dependencies/), [continuous deployment](https://docs.netlify.com/build/configure-builds/overview/).

## Google Analytics 4

The existing adapter supports GA4; analytics is disabled by default.

1. Create a GA4 property and a Web data stream for the production site. Copy the `G-...` measurement ID from Admin → Data streams → your Web stream ([Google instructions](https://support.google.com/analytics/answer/12270356?hl=en)).
2. Set the analytics section of `config/site.json`:

```json
"analytics": {
  "enabled": true,
  "measurement_id": "G-MGL3FX7522",
  "production_host": "central-texas-routes-map.netlify.app"
}
```

3. Keep `production_url` and `analytics.production_host` aligned if using a custom domain. Build and deploy in production mode.
4. On the production host, select Allow in Privacy & Analytics, interact with the catalog, and check events in GA4 Realtime. Decline must prevent event collection. Local and development preview builds do not collect analytics.

The adapter sends sanitized catalog interaction events (including route opens, filters, navigation clicks and GPX downloads). Automatic page views are disabled. Do not add a second Google tag through Netlify or HTML, which could bypass the app’s consent gating. The configured measurement ID is `G-MGL3FX7522`; collection still requires a production deployment and visitor consent.

## Catalog

Distances below come from the current authored route records; the live app remains the place to inspect route details.

| Route | Category | Miles |
|---|---|---:|
| Austin to Llano - Hill Country Blast | Twisties & Canyons | 179.7 |
| Balcones Escarpment Twisties | Twisties & Canyons | 29.1 |
| Bertram, RM 1174 & RM 963 Loop | Plains & Ranchlands | 54.7 |
| Blanco River Valley & FM 165 Loop | Lakes & Rivers | 70.6 |
| Briggs, Oakalla & RM 963 Loop | Plains & Ranchlands | 23.1 |
| Devil's Backbone Scenic Ridge | Twisties & Canyons | 34.3 |
| Fitzhugh & Hamilton Pool Craft Beverage Run | German Towns & Culture | 15.9 |
| German Heritage Trail | German Towns & Culture | 61.2 |
| Guadalupe River Road & Hunt Sweep | Lakes & Rivers | 54.6 |
| Highland Lakes Circuit | Lakes & Rivers | 43.2 |
| Kingsland & Lake LBJ Loop | Lakes & Rivers | 47.5 |
| Lake Victor & Burnet Backcountry Sweep | Plains & Ranchlands | 25.0 |
| Leander to Luckenbach Loop | German Towns & Culture | 219.5 |
| Lime Creek Road & Lake Travis Loop | Twisties & Canyons | 26.7 |
| Llano River & Castell Run | Lakes & Rivers | 43.2 |
| Lost Pines & Bastrop State Park Loop | Plains & Ranchlands | 27.8 |
| Mason Mountain Ridge Run | Plains & Ranchlands | 57.3 |
| North Gabriel River Sweep | Plains & Ranchlands | 22.1 |
| Oakalla & Berry Creek Valley Run | Plains & Ranchlands | 35.5 |
| Pedernales Wine Country Sweep | German Towns & Culture | 77.7 |
| Round Top & Fayetteville Country Lane | Plains & Ranchlands | 28.3 |
| Saturday Dawn Lake Granger Loop | Lakes & Rivers | 105.3 |
| Saturday Dawn Lake Travis Loop | Lakes & Rivers | 99.0 |
| Sisterdale, Kendalia & Luckenbach Run | German Towns & Culture | 54.0 |
| The Three Twisted Sisters Circuit | Twisties & Canyons | 174.0 |
| Willow City Loop & Enchanted Rock Run | Twisties & Canyons | 35.7 |

## License

MIT — Copyright (c) 2026 Vadym Fedorov.
