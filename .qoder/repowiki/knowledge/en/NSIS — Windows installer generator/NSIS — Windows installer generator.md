---
kind: external_dependency
name: NSIS — Windows installer generator
slug: nsis
category: external_dependency
category_hints:
    - vendor_identity
scope:
    - '**'
source_files:
    - src-tauri/tauri.conf.json
---

### Identity
Nullsoft Scriptable Install System, invoked by Tauri's Windows bundling to produce an `.exe` setup package.

### Role in this repo
Tauri's `bundle.targets = "all"` produces both MSI and NSIS installers under `src-tauri\target\release\bundle\`. On Windows the NSIS output is `FreeSpace_0.1.0_x64-setup.exe`.

### Durable usage model
NSIS is a Tauri-provided bundler target, not wired directly in Cargo; it is downloaded automatically on first Windows release build when internet access is available. No custom NSIS script lives in this repo — the installer is generated from Tauri defaults plus the icon set in `src-tauri/icons/`.

Verify exact API/params against official docs.