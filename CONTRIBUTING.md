# Contributing to SchemaBear

Thanks for helping improve SchemaBear.

## Development

```bash
npm install
npm run tauri dev
```

Run the quality gate before opening a PR:

```bash
make check
```

## Project rules

- Keep `er-core` pure Rust (no Tauri/UI deps).
- Prefer small, tested parsers over large dependency stacks.
- Update fixtures when interchange formats change.
- Keep `AGENTS.md` ≤ 70 lines; put deep docs in `README.md`.

## Commit style

- Imperative subject: `Add DBML composite ref support`
- Group related changes; avoid drive-by refactors

## License

By contributing, you agree your changes are licensed under the MIT License  
(copyright flyingbear / flyingbearHK).
