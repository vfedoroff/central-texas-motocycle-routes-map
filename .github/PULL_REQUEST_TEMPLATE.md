## Summary of Changes

<!-- Briefly describe the routes, places, or improvements added in this PR. -->

## Authoring Standards Checklist

- [ ] **Paved Surfaces**: Confirmed that all route coordinates travel over 100% paved surfaces (asphalt or concrete).
- [ ] **No Parking Lots / Spurs**: Start and end coordinates are directly on public road centerlines, not inside gas stations, private driveways, or parking lots.
- [ ] **Loop Closure**: For loop routes, start and end coordinates match exactly.
- [ ] **Factual Sourcing**: Added authoritative source references with valid `https://` URLs and access dates.
- [ ] **Photo Rights**: Any submitted photo includes explicit `credit` and an approved open license (or no photos added).
- [ ] **No Generated Files**: No files from `dist/`, `.build/`, or generated downloads are included in this PR.

## Local Verification Commands

Please run and confirm all commands succeed:

- [ ] `cargo xtask check` (rustfmt, clippy, workspace tests)
- [ ] `cargo xtask validate` (schema and zero-spur topology audit)
- [ ] `cargo xtask build --development` (development preview build)
- [ ] Local preview inspected with `npm run preview`
