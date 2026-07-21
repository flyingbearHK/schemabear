# Active Context

Lifecycle: active  
Confidence: confirmed  
Last verified: 2026-07-22  
Source of truth: `README.md`, `crates/er-core`, `src-tauri`

## Resume

- Product is **ER Diagram**: Tauri 2 + Rust on macOS arm64.
- Interchange: **Mermaid erDiagram** (AI-friendly in) and **DBML** (dbdiagram.io out).
- Core library `er-core` is pure Rust and covered by unit + fixture tests.
- UI is vanilla TS/SVG (no heavy frontend framework) for small bundle size.
- Quality gate: `make check` (er-core tests, tauri check, frontend build, cargo build).

## Open Questions

- Whether to add SQL DDL export next vs. undo stack in UI.

## Do Not

- Do not treat the HMS sample as a certified production schema.
- Do not add UI frameworks without a clear size/perf reason.
