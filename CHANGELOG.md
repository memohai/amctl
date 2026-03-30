# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] (target: 0.2.1)

### Added
- On-demand refs diff in REST refs flow: introduce UI-change sequence tracking from accessibility events and skip full refs recompute when cache is still valid.
- Server-side ref alias replay mapping: persist `ref -> token` at observe time and resolve by token at act time.

### Changed
- `POST /api/nodes/tap` with `by=ref` now resolves ref aliases through server-side mapping instead of client-sent version checks.
- Ref replay resolution now uses exact token first, then identity-token fallback only when there is a unique candidate.
- CLI no longer sends `expected_ref_version` for `by=ref` and no longer sends `known_ref_version` for `/api/screen/refs`.
- Refs auto-refresh now runs only when ref panel is visible, reducing unnecessary background recomputation.

### Fixed
- Add dedicated decision tests for refs rebuild gating to reduce regression risk.
- Add dedicated tests for ref alias mapping and stale/ambiguous fallback behavior.

## [0.2.0] - 2026-03-28

Range: `app-v0.1.1-rc.1` (`06bf830`) .. `app-v0.2.0` (`6201e4b`)

### Added
- Versioned refs replay and on-screen ref labels for safer ref-based actions (`8a6e4d5`).
- NPM meta package and platform package publish pipeline for CLI distribution (`626eb23`).

### Changed
- Rename app package to `com.memohai.autofish` (`d9328c4`).
- Rename CLI command from `amc` to `af`, with environment variable alignment (`ce72bd9`).
- Align docs and workflows with Autofish rename (`f145559`).

### Fixed
- Fix npm package scope alignment to `@memohjs` across loader/docs/workflow (`eea0eb6`).
- Fix npm Linux install path during RC progression (`0eee441`).
- Fix CLI package version issues (`92163e3`).
- Fix release archive naming to avoid duplicate prefix (`187015d`).

## [0.1.1-rc.1] - 2026-03-24

Range: project bootstrap (`547a61a`) .. `app-v0.1.1-rc.1` (`06bf830`)

### Added
- Initial architecture/design docs and agent guidelines (`547a61a`, `f38952a`).
- Android direct control path via Shizuku (`eb9a83b`).
- REST API server with separate port and bearer auth (`b29583f`).
- Android settings/log UI improvements and startup/log visibility work (`6cad620`).
- Deterministic executor support in CLI (`9d17003`).
- Migration to Rust-based CLI with broader control reliability improvements (`8f273d6`).
- CLI request API refactor and screen output improvements (`a38ef8e`).
- SQLite trace persistence for CLI command history (`ff78df0`).
- Observe/verify UX and guidance improvements (`f7659ac`).
- Overlay offset config and multi-color legend support (`1c81672`).
- Tuple-style swipe arguments and clearer action help (`125f267`).

### Changed
- Refactor CLI runtime/command shaping during early evolution (`77c29eb`).
- Hide legacy UI surfaces and unify service wording/runtime IP display (`3e4dc03`).
- Simplify architecture docs and refresh root/CLI readmes (`8125900`).

### Fixed
- Async server stop behavior and accessible full-row toggle in UI (`f956d6d`).
- Validate non-negative tap coordinates and align `node-exists` selector behavior (`6bd9f3a`).
- Prevent startup theme flicker and splash mismatch (`f3dea24`, `6f08d19`).

### Performance
- Optimize release profile for smaller CLI binary size (`482ca17`).

### Build/CI/Release
- Prefer official Gradle repos with mirror fallback (`77c6f89`).
- Split release workflows and update GitHub Actions compatibility (`1a0369a`, `c6cc16b`, `62cf6f7`, `6e7809e`).
- Add license and publish metadata updates (`25ad62b`, `ea3c18d`).
- Tag and release bump to `0.1.1-rc.1` (`06bf830`).
