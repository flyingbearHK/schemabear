.PHONY: check test lint build dev app clean fmt

check: test build
	@echo "✓ quality gate passed"

test:
	cargo test --manifest-path crates/er-core/Cargo.toml
	cargo check --manifest-path src-tauri/Cargo.toml

lint: fmt
	cargo clippy --manifest-path crates/er-core/Cargo.toml --all-targets -- -D warnings || \
	  cargo check --manifest-path crates/er-core/Cargo.toml

fmt:
	cargo fmt --manifest-path crates/er-core/Cargo.toml
	cargo fmt --manifest-path src-tauri/Cargo.toml

build:
	npm run build
	cargo build --manifest-path src-tauri/Cargo.toml

dev:
	npm run tauri dev

app:
	npm run tauri:build:mac

app-win:
	npm run tauri:build:win

clean:
	rm -rf dist node_modules target src-tauri/target crates/er-core/target
