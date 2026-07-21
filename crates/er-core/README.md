# er-core

Pure Rust entity-relationship model and interchange library used by **SchemaBear**.

## Capabilities

- In-memory `Diagram` model (entities, attributes, relationships, positions)
- Mermaid `erDiagram` import/export
- DBML import/export (dbdiagram.io)
- Deterministic auto-layout
- Structural validation

## Use

```rust
use er_core::{import_mermaid, export_dbml, auto_layout, validate};

let mut diagram = import_mermaid(include_str!("../../../fixtures/infor_hms_sample.mmd"))?;
auto_layout(&mut diagram, true);
assert!(validate(&diagram).ok);
let dbml = export_dbml(&diagram);
```

No UI or Tauri dependencies — safe for CLI, WASM, or server reuse.
