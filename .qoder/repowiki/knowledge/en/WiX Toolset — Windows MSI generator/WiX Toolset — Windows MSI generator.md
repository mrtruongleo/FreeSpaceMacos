---
kind: external_dependency
name: WiX Toolset — Windows MSI generator
slug: wix-toolset
category: external_dependency
category_hints:
    - vendor_identity
scope:
    - '**'
source_files:
    - src-tauri/tauri.conf.json
---

### Identity
Microsoft WiX Toolset, invoked by Tauri's Windows bundling to produce an `.msi` package.

### Role in this repo
Produced alongside NSIS under `src-tauri\target\release\bundle\msi\` as `FreeSpace_0.1.0_x64_en-US.msi`. Like NSIS, WiX is a Tauri-provided target and is auto-downloaded on first Windows release build.

### Durable usage model
No custom WiX `.wxs` files are present in the repo; the MSI is generated from Tauri defaults. Adding a new Windows feature requires updating Tauri configuration or Rust-side metadata rather than editing a WiX script.

Verify exact API/params against official docs.