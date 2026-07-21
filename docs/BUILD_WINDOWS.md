# Building SchemaBear on Windows

SchemaBear is a [Tauri 2](https://tauri.app) app. **Windows installers must be built on a Windows machine** (or CI). Cross-compiling the full Tauri bundle from macOS/Linux is not supported.

Supported Windows targets:

| Artifact | Description |
|----------|-------------|
| `.exe` (NSIS) | Recommended installer — `SchemaBear_x.y.z_x64-setup.exe` |
| `.msi` | MSI package for enterprise/deploy tools |
| bare `.exe` | Unbundled binary under `src-tauri\target\release\` |

## 1. Prerequisites

### Required

1. **Windows 10 (1803+) or Windows 11** — 64-bit
2. **[Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)**  
   - Install “Desktop development with C++”  
   - Or full Visual Studio 2022 with the MSVC toolchain
3. **[Rust](https://rustup.rs/)** (stable-msvc)  
   ```powershell
   # After installing rustup-init.exe (choose MSVC toolchain):
   rustc -V
   cargo -V
   rustup default stable-x86_64-pc-windows-msvc
   ```
4. **[Node.js 20+](https://nodejs.org/)** (LTS recommended)  
   ```powershell
   node -v
   npm -v
   ```
5. **WebView2 Runtime**  
   - Usually preinstalled on Windows 11 and recent Windows 10  
   - If missing: [Evergreen Standalone Installer](https://developer.microsoft.com/microsoft-edge/webview2/)  
   - SchemaBear’s installer can also bootstrap WebView2 (`downloadBootstrapper`)

### Optional but helpful

- **Git for Windows**
- Windows Terminal / PowerShell 7+
- `cargo-binstall` or just use `npm`’s bundled `@tauri-apps/cli`

## 2. Clone and install

```powershell
git clone https://github.com/flyingbearHK/schemabear.git
cd schemabear
npm install
```

## 3. Develop (hot reload)

```powershell
npm run tauri dev
```

This starts Vite on port `1420` and opens the SchemaBear window.

## 4. Quality checks (no installer)

```powershell
npm run test:unit
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

Or on machines with `make` (e.g. Git Bash):

```bash
make check
```

## 5. Release build (Windows x64)

From the repo root **in PowerShell or cmd**:

```powershell
npm run tauri:build:win
```

Equivalent:

```powershell
npx tauri build --target x86_64-pc-windows-msvc
```

### Output locations

```text
src-tauri\target\x86_64-pc-windows-msvc\release\schemabear.exe

src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\
  SchemaBear_0.x.y_x64-setup.exe

src-tauri\target\x86_64-pc-windows-msvc\release\bundle\msi\
  SchemaBear_0.x.y_x64_en-US.msi
```

If you omit `--target`, Tauri builds for the host triple and writes under:

```text
src-tauri\target\release\bundle\
```

## 6. ARM64 Windows (optional)

On a Windows on ARM device (or with the ARM64 MSVC toolchain installed):

```powershell
rustup target add aarch64-pc-windows-msvc
npx tauri build --target aarch64-pc-windows-msvc
```

## 7. Common problems

| Symptom | Fix |
|---------|-----|
| `link.exe not found` | Install VS Build Tools with MSVC + Windows SDK |
| `WebView2 not found` | Install WebView2 Evergreen Runtime |
| `failed to run 'npm run build'` | Run `npm install` first; use Node 20+ |
| Antivirus locks `schemabear.exe` | Exclude the `target` folder during builds |
| Slow first build | Normal — Rust release + LTO compiles once |

## 8. CI artifacts

GitHub Actions builds Windows NSIS/MSI on every push to `main` and uploads them as workflow artifacts:

**Actions → CI → latest run → Artifacts → `schemabear-windows-x64`**

You can also download release assets when a GitHub Release is published.

## 9. Signing (optional, production)

For SmartScreen-friendly installs, sign the NSIS/MSI with a code-signing certificate after build (e.g. `signtool sign ...`). Unsigned builds still run locally; users may see a SmartScreen warning on first open.

## See also

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- macOS build: `npm run tauri:build:mac` (see main [README](../README.md))
