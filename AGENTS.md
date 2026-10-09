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

Give each new route a reasonably distinct map color. Check existing route colors and choose a shade that is easy to distinguish, especially from nearby or overlapping routes; avoid automatically reusing the category color.

## Git and release

Create signed Conventional Commits when requested. Push only when authorized. Internal docs, QA records and plans remain local and ignored. Netlify builds production output from main using netlify.toml.
