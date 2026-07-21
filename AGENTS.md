# 843-Pi — ER Diagram

Small Tauri + Rust ER studio for macOS Apple Silicon. Mermaid in, DBML out, pure `er-core`.

## Stack

- UI: Vite + TypeScript + SVG (`src/`)
- Shell: Tauri 2 (`src-tauri/`)
- Core: Rust crate `crates/er-core` (Mermaid/DBML/layout/validate)
- Package managers: npm + cargo

## Commands

```bash
npm install
npm run tauri dev          # app
make check                 # quality gate
cargo test --manifest-path crates/er-core/Cargo.toml
npm run tauri:build        # aarch64-apple-darwin bundle
```

## Architecture

```
src/  → invoke → src-tauri/commands → er-core
fixtures/mohg_hms_sample.mmd = hospitality sample
```

Dependency direction: UI → commands → `er-core` → (no UI deps). Extend formats inside `er-core` first.

## Constraints

- Keep `er-core` pure (no Tauri).
- Prefer DBML as primary interchange with mainstream ER tools; Mermaid for AI/docs.
- Root `AGENTS.md` ≤70 lines; deep docs in README.
- No secrets; don't commit `target/`, `dist/`, `node_modules/`.
- Run `make check` before claiming done.

## Memory / tasks

- Resume: `START_HERE.md` → `memory/ACTIVE_CONTEXT.md` → `tasks/todo.md`
- Sample semantics are illustrative, not official Infor/MOHG schema.

## Nav

`README.md` · `crates/er-core` · `fixtures/` · `Makefile`
