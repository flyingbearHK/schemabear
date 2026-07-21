# Active Context

Lifecycle: active  
Confidence: confirmed  
Last verified: 2026-07-22  
Source of truth: `README.md`, `crates/er-core`, `src-tauri`

## Resume

- Product: **SchemaBear** by **flyingbear** (`flyingbearHK/schemabear`).
- Platforms: macOS arm64 + Windows x64 (NSIS/MSI); Windows built on Windows/CI only.
- Docs: `docs/BUILD_WINDOWS.md`; CI uploads `schemabear-windows-x64` artifacts.
- Interchange: Mermaid in, DBML out; visual editor + code path.
- Sample: **Infor HMS Sample**.
- Quality gate: `make check` / `npm run check`.

## Do Not

- Do not treat the HMS sample as a certified production schema.
