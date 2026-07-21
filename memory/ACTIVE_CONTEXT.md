# Active Context

Lifecycle: active  
Confidence: confirmed  
Last verified: 2026-07-22  
Source of truth: `README.md`, `crates/er-core`, `src-tauri`

## Resume

- Product is **ER Diagram**: Tauri 2 + Rust on macOS arm64 (v0.1.1).
- Interchange: **Mermaid erDiagram** (AI-friendly in) and **DBML** (dbdiagram.io out).
- **Visual editor** + code path: Edit tab for entities/attrs/rels; Code tab for Mermaid/DBML.
- Zoom: on-canvas controls, scroll-zoom, keyboard `+`/`-`/`0`.
- Layout: relationship-aware layered placement in `er-core`.
- Quality gate: `make check`.

## Open Questions

- Whether to add SQL DDL export next vs. undo stack in UI.

## Do Not

- Do not treat the HMS sample as a certified production schema.
- Do not add UI frameworks without a clear size/perf reason.
