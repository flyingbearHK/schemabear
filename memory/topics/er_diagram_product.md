# Topic: ER Diagram Product

Lifecycle: active  
Confidence: confirmed  
Last verified: 2026-07-22  
Source of truth: `README.md`, `crates/er-core`

## Facts

- App identifier: `com.erdiagram.app`
- Primary platforms target: macOS Apple Silicon (`aarch64-apple-darwin`)
- Import: Mermaid `erDiagram`, DBML, JSON
- Export: DBML (primary mainstream), Mermaid, JSON
- Sample: `fixtures/mohg_hms_sample.mmd` — PROPERTY, GUEST, RESERVATION, stay night, folio path

## Decisions

- DBML chosen over SQL/PlantUML as first export because of dbdiagram.io popularity and simple text ergonomics.
- Pure `er-core` crate keeps future CLI/WASM expansion cheap.
