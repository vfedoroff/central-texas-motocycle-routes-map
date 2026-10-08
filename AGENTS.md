# Agent Guide: Managing Ride Atlas Routes and Catalog

This document is the authoritative guide for AI agents and developers working on the Ride Atlas catalog. It details the project architecture, strict standards, environment setup, canonical commands, authored schemas, and step-by-step authoring recipes.

---

## 1. Project Architecture

The repository uses a **modular Rust catalog architecture**:

```
.
├── content/                         # Authored source of truth (committed)
│   ├── routes/                      # Route metadata records (*.json)
│   ├── geometry/routes/             # Snapped route coordinates (*.geojson)
│   ├── places/                      # Points of interest / POIs (*.json)
│   ├── roads/                       # Individual road segments (*.json)
│   ├── network-paths/routes/        # Verified topological graph paths (*.json)
│   ├── evidence/routes/             # Road audit evidence records (*.json)
│   └── photos/                      # Project-owned, licensed media assets
├── crates/
│   ├── catalog-model/               # Domain models, schema deserialization, validation
│   ├── catalog-roads/               # Road graph topology, policy, alignment, evidence
│   ├── catalog-build/               # Static site generator and staging pipeline
│   ├── catalog-ui/                  # Interactive Leptos WASM app & Leaflet adapter
│   ├── route-lint/                  # Read-only catalog & road verification CLI
│   └── xtask/                       # Task runner (`cargo xtask`)
├── config/                          # Site and road audit configurations
├── data/road-network/               # Compressed reference road network graph (*.zst)
├── tests/browser/                   # Playwright end-to-end browser tests
├── dist/                            # Generated static build output (git-ignored)
├── mise.toml                        # Pinned dev environment tools
└── README.md                        # Public documentation
```

### Authored Inputs vs. Generated Artifacts
- **Authored files** live exclusively in `content/` and `config/`. Agents edit these JSON/GeoJSON files directly.
- **Generated outputs** live in `dist/` and `.build/`. They are compiled deterministically by `catalog-build` and `trunk`. **Never edit or commit files in `dist/`**.
- **No Agent Commits or Deployments**: Under this implementation plan, all modifications must remain **uncommitted in the local working tree** for human owner review. Never run `git commit`, `git push`, or deploy commands.

---

## 2. Core Route Standards & Quality Rules

When curating or editing a route:
1. **Paved Surfaces Only**:
   - Must be 100% paved (asphalt or concrete) and suitable for street motorcycles.
   - Gravel, dirt, and caliche roads are strictly excluded.
2. **Loop or Scenic Corridor**:
   - Prefer circular day-loops (15–180 miles) or scenic riverside/ridge corridors.
3. **Zero Dead-End Spurs or Parking Lot Incursions**:
   - **Never start or terminate inside a gas station, business parking lot, or private driveway**.
   - Loops must begin and end directly on public highway/road centerline intersections.
   - Prune accidental out-and-back ranch road spurs, cul-de-sac diversions, and neighborhood loops.
4. **Snapped to Road Centerlines**:
   - Geometries must follow actual highway and road centerlines.
   - For closed loops, verify that the first and last coordinates match identically.
5. **Zero-Spur Topology Requirement**:
   - Every loop must pass an automated topological spur audit with 0 unwanted spurs (`cargo xtask validate`).
6. **Factual Evidence & Sourcing**:
   - Sources must include valid `https://` URLs to official transport departments or park authorities with the ISO access date (`YYYY-MM-DD`).
   - Never guess node IDs, reviewer identities, or personal riding claims. Missing evidence means `review_required`, not `pass`.

---

## 3. Developer Environment Bootstrap

Development tools are pinned with [mise](https://mise.jdx.dev/):

```sh
# 1. Trust and install pinned toolchains (Rust 1.99.0, Node 24.21.0, Trunk 0.21.14)
mise trust
mise install

# 2. Configure toolchain components (wasm32 target, clippy, rustfmt)
mise run setup
```

---

## 4. Canonical Commands & Operations

### Read-Only Inspection Operations
- `cargo xtask check`: Runs `rustfmt --check`, `clippy -D warnings`, and all workspace tests.
- `cargo xtask validate`: Validates all authored JSON schemas, referential integrity, and route topology (zero spurs).
- `cargo run --locked -p route-lint -- --route <id> --format json`: Runs isolated catalog and road verification for a specific route with structured JSON diagnostics.
- `npm run test:browser`: Runs the full Playwright browser test suite across desktop and mobile viewports.

### Explicit Writing Operations
- `cargo xtask network-restore`: Decompresses and validates the pinned reference road network graph (`data/road-network/graph.json`).
- `cargo xtask audit-roads --route <id>`: Runs the road network audit and writes verified evidence to `content/evidence/routes/<id>.json`.
- `cargo xtask build --development`: Performs an atomic static site compilation in development mode (allows unverified previews, written to `dist/`).
- `cargo xtask build`: Performs a production static site build. Road-network audits are optional and must not block publication.
- `npm run preview`: Launches the local static preview server on `http://127.0.0.1:8000`.

---

## 5. Recipe: How to Add a New Route

Follow these steps in order when authoring a new route:

### Step 1: Choose Route ID, Category, and Color
Generate a unique `kebab-case` ID (e.g. `blanco-river-valley-fm-165-loop`). Select one of the 4 canonical categories:

| Category | Hex Color | Focus |
|---|---|---|
| `Twisties & Canyons` | `#e74c3c` | Elevation changes, switchbacks, technical turns |
| `Lakes & Rivers` | `#2980b9` | Highland lakes, river corridors, dam crossings |
| `German Towns & Culture` | `#8e44ad` | Historic trails, dancehalls, heritage towns |
| `Plains & Ranchlands` | `#27ae60` | Post-oak pastures, open ranch country sweepers |

### Step 2: Author Geometry (`content/geometry/routes/<id>.geojson`)
Create `content/geometry/routes/<id>.geojson` with a single LineString feature. Coordinates are `[longitude, latitude]`:
- For loops: ensure `coordinates[0] == coordinates[last]`.
- Start/end directly on public road intersections, not parking lots.

### Step 3: Author Route Metadata (`content/routes/<id>.json`)
Create `content/routes/<id>.json` matching the schema in `docs/content-schema.md`:
- Derive exact `distance_mi`.
- Define descriptive `waypoints`, `via`, and category tags.
- Add authoritative `sources` with URLs and `accessed_on` dates.
- Optional: link route `stops` by referencing existing POI IDs (`place_id`).

### Step 4: Topological Path & Road Audit
1. Add `content/network-paths/routes/<id>.json` referencing real graph edges and verified public junction nodes.
2. Restore reference network if not already present: `cargo xtask network-restore`.
3. Audit route against road network: `cargo xtask audit-roads --route <id>`.
4. Validate with route-lint: `cargo run --locked -p route-lint -- --route <id> --format json`.
5. Run full workspace validation: `cargo xtask validate`.
6. Compile and inspect preview: `cargo xtask build --development && npm run preview`.

---

## 6. Recipe: How to Add a Point of Interest (POI)

POIs represent landmarks, fuel stops, food, scenic overlooks, or historic destinations.

1. **Choose Unique ID**: Use `kebab-case` (e.g. `luckenbach-general-store`).
2. **Create Place Record (`content/places/<id>.json`)**:
   - Required fields: `schema_version: 1`, `id`, `title`, `summary`, `place_category`, and `coordinates: [lon, lat]`.
   - Valid categories: `scenic_overlook`, `historic_site`, `fuel_stop`, `food_drink`, `campground`, `park`, `water_crossing`, `general`.
   - Optional fields: `description` (safe Markdown), `photos`, `sources`, `author_note`, `related` (other `ObjectKey`s).
3. **Optional Route Linkage**: If this POI is an authored stop on a route, add it to that route's `stops` array in `content/routes/<route-id>.json`.
4. **Validation**: POIs do not require GPX files, road paths, or centerline audits. Run `cargo xtask validate` and `cargo xtask build --development`. Inspect the marker, details panel, and Google Maps handoff in preview.

---

## 7. Editing & Deletion Behavior

- **Stable Identifiers**: Object IDs are stable primary keys. Never rename an ID without updating all referencing `stops` and `related` entries.
- **Cache Invalidation**: Any edit to a route's geometry (`.geojson`), network path (`.json`), or audit configuration invalidates its existing road audit evidence. You must re-run `cargo xtask audit-roads --route <id>`.
- **Deleting Objects**: When deleting a POI, remove all references to it in route `stops` arrays to prevent broken foreign key diagnostics.
- **Reporting Contribution State**: Always conclude contributions by summarizing changed source paths, actual validation command outputs, pending factual evidence, and local preview status.
