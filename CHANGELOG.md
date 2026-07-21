# Changelog

All notable changes to this project are documented in this file.

## [0.1.5] — 2026-07-22

### Added

- Windows packaging targets (NSIS installer + MSI)
- `docs/BUILD_WINDOWS.md` — full Windows build guide
- CI builds for Windows x64 and macOS arm64 with downloadable artifacts
- npm scripts: `tauri:build:mac`, `tauri:build:win`, `tauri:build:win-arm`

## [0.1.4] — 2026-07-22

### Changed

- Product renamed to **SchemaBear** by flyingbear
- Sample renamed to **Infor HMS Sample** (removed MOHG branding)
- Bundle id `hk.flyingbear.schemabear`

### Fixed

- Canvas drag performance (incremental SVG updates)
- Text-selection highlight while panning

## [0.1.2] — 2026-07-22

### Added

- Prominent **Auto Arrange** / canvas **Arrange** buttons for one-click layout
- Theme switcher: **System**, **Day** (light), **Dark** (persisted)

### Improved

- Auto-layout barycenter ordering to reduce relationship crossings and align chains

## [0.1.1] — 2026-07-22

### Added

- Visible zoom controls (+ / − / % / Fit) plus scroll-to-zoom and keyboard shortcuts
- Visual editor (not code-only): add/rename/delete entities, edit attributes & flags, add/delete relationships
- Relationship-aware layered auto-layout (one-side left of many-side)
- Rounded orthogonal relationship connectors with clearer crow’s-foot markers

## [0.1.0] — 2026-07-22

### Added

- Initial Tauri + Rust ER diagram studio for macOS Apple Silicon
- Pure `er-core` library: model, Mermaid ER import/export, DBML import/export, layout, validation
- SVG canvas with pan/zoom, drag, crow’s-foot markers
- Code panel for AI-friendly Mermaid/DBML editing
- Infor HMS inspired sample fixture
- MIT license, Makefile quality gate, OSS layout
