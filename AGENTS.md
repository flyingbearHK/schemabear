# SchemaBear

Small Tauri + Rust ER studio for macOS Apple Silicon by **flyingbear**.  
Mermaid in, DBML out, pure `er-core`.

## Stack

- UI: Vite + TypeScript + SVG (`src/`)
- Shell: Tauri 2 (`src-tauri/`) — product **SchemaBear**
- Core: Rust crate `crates/er-core`
- Package managers: npm + cargo

## Commands

```bash
npm install
npm run tauri dev
make check
cargo test --manifest-path crates/er-core/Cargo.toml
npm run tauri:build
```

## Architecture

```
src/  → invoke → src-tauri/commands → er-core
fixtures/infor_hms_sample.mmd = hospitality sample
```

Dependency direction: UI → commands → `er-core` → (no UI deps).

## Constraints

- Keep `er-core` pure (no Tauri).
- Prefer DBML as primary interchange; Mermaid for AI/docs.
- Root `AGENTS.md` ≤70 lines.
- No secrets; don't commit `target/`, `dist/`, `node_modules/`.
- Run `make check` before claiming done.
- Sample is illustrative Infor HMS — not a certified production schema.

## Nav

`README.md` · `crates/er-core` · `fixtures/` · `Makefile`
