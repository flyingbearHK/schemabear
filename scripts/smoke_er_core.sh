#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> er-core tests"
cargo test --manifest-path crates/er-core/Cargo.toml --quiet

echo "==> fixture round-trip via rustc example"
cargo test --manifest-path crates/er-core/Cargo.toml mohg_sample -- --nocapture --quiet

echo "==> frontend typecheck/build"
npm run build --silent

echo "==> tauri/rust check"
cargo check --manifest-path src-tauri/Cargo.toml --quiet

echo "✓ smoke ok"
