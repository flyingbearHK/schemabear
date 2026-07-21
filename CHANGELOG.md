# Changelog

All notable changes to this project are documented in this file.

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
- MOHG / Infor HMS inspired sample fixture
- MIT license, Makefile quality gate, OSS layout
