# Contributing to Ride Atlas

Thank you for your interest in contributing to the Central Texas Ride Atlas! This guide explains how to add routes, places of interest, and improvements while maintaining our strict standards for safety, accuracy, and code quality.

---

## 1. Guiding Principles & Route Standards

1. **Paved Surfaces Only**: Every route and road segment must be fully paved (asphalt or concrete) and suitable for street motorcycles. Gravel, caliche, and dirt county roads are strictly excluded.
2. **Loop or Scenic Corridor**: Day rides should either be continuous loops (15–180 miles) or scenic corridors connecting key regions.
3. **No Parking Lot or Private Driveway Incursions**:
   - Loops must start and terminate directly on public road centerline intersections.
   - Do **not** route into gas station forecourts, restaurant parking lots, or residential driveways. Stops are modeled separately as POIs (`content/places/`).
4. **Zero-Spur Topology**: All loops must be free of accidental out-and-back spurs or cul-de-sac detours (`cargo xtask validate`).
5. **Factual Sourcing**: All claims, roads, and places must include factual references (e.g. TxDOT highway records, state park maps) with access dates.
6. **No Unlicensed Photography**: Only submit photos you took yourself or that are licensed under an open license (`CC-BY-4.0`, `CC0`).

---

## 2. Setting Up Your Development Environment

We use [mise](https://mise.jdx.dev/) to pin identical Rust, Node.js, and Trunk toolchains:

```sh
# Trust and install pinned tools
mise trust
mise install

# Set up toolchain components (wasm32 target, clippy, rustfmt)
mise run setup
```

---

## 3. Authored Files vs. Generated Outputs

- **Edit only files in `content/`**:
  - `content/routes/<id>.json`
  - `content/geometry/routes/<id>.geojson`
  - `content/places/<id>.json`
  - `content/roads/<id>.json`
  - `content/network-paths/routes/<id>.json`
- **Never edit or commit generated outputs**:
  - `dist/` is generated and git-ignored.
  - Do not manually author GPX or index files.

---

## 4. Verification Checklist

Before submitting a Pull Request, verify that all automated checks pass locally:

```sh
# 1. Format and Clippy checks
cargo xtask fmt
cargo xtask clippy

# 2. Workspace unit and integration tests
cargo xtask test

# 3. Catalog and topology audit (zero spurs)
cargo xtask validate

# 4. Optional: run route-lint on your specific route
cargo run --locked -p route-lint -- --route <your-route-id> --format json

# 5. Local preview build
cargo xtask build --development
npm run preview
```

See [AGENTS.md](AGENTS.md) for the authoring workflow and the existing records in `content/` for schema examples.
