---
kind: external_dependency
name: Tauri 2 — cross-platform desktop shell
slug: tauri
category: external_dependency
category_hints:
    - vendor_identity
scope:
    - '**'
source_files:
    - src-tauri/Cargo.toml
    - src-tauri/tauri.conf.json
    - src-tauri/capabilities/default.json
---

### Identity
Tauri 2 is the desktop application framework wrapping the Rust backend (`freespace_lib`) and the TypeScript/Vite frontend.

### Role in this repo
- `src-tauri/Cargo.toml` declares `tauri = "2"` and `tauri-plugin-opener = "2"`; `tauri.conf.json` configures the app window, CSP, bundle targets, and icons.
- Frontend calls into the Rust backend through Tauri's invoke system; the opener plugin is granted via capabilities and used to reveal files in Explorer/Finder.
- `npm run tauri build` drives the full pipeline: Vite → Rust release → MSI + NSIS installers.

### Durable integration shape

Verify exact API/params against official docs.