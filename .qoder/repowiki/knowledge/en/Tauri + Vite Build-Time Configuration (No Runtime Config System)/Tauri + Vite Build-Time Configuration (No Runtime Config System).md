---
kind: configuration_system
name: Tauri + Vite Build-Time Configuration (No Runtime Config System)
category: configuration_system
scope:
    - '**'
source_files:
    - src-tauri/tauri.conf.json
    - vite.config.ts
    - package.json
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/capabilities/default.json
---

## Overview

This FreeSpace macOS desktop app does not implement a runtime configuration system. There is no `.env` file, no config loader, no feature flags, and no user-editable settings file. All application behavior is configured at **build time** through Tauri's `tauri.conf.json`, Vite's `vite.config.ts`, and Cargo's `Cargo.toml`. The frontend (`src/main.ts`) contains no configuration loading logic — it hard-codes UI strings, default paths, and behavior.

## Key Configuration Files

- `src-tauri/tauri.conf.json` — Tauri v2 app manifest: product name, version, identifier (`com.freespace.macos`), window geometry, CSP policy, bundle targets, and icons.
- `vite.config.ts` — Vite dev server port (`1420`), HMR host/port (`1421`), and `ignored` watch rules for `src-tauri`.
- `package.json` — npm scripts (`dev`, `build`, `preview`, `tauri`) and dependencies (`@tauri-apps/api`, `@tauri-apps/plugin-opener`).
- `src-tauri/Cargo.toml` — Rust crate metadata, Tauri v2 dependency, and platform-specific deps (`libc` on macOS, `windows-sys` on Windows).
- `src-tauri/capabilities/default.json` — Tauri v2 capability granting `core:default` and `opener:default` permissions to the `main` window.
- `src-tauri/build.rs` — thin wrapper calling `tauri_build::build()`.

## Architecture & Conventions

1. **Build-time only**: `tauri.conf.json` declares `beforeDevCommand: "npm run dev"` and `beforeBuildCommand: "npm run build"`, with `frontendDist: "../dist"` and `devUrl: "http://localhost:1420"`. The frontend and backend are built together by Tauri; there is no separate runtime config layer between them.

2. **Environment variables are minimal and dev-only**: `vite.config.ts` reads `process.env.TAURI_DEV_HOST` to adjust HMR host/port during development. No other env vars are consumed anywhere in the codebase.

3. **No runtime settings API**: The Rust backend (`src-tauri/src/lib.rs`, `src-tauri/src/main.rs`) exposes Tauri commands (`scan_system`, `delete_items`, `scan_sub_items`, `get_tree_root`, etc.) but has no config-loading code. Paths and behavior are derived from OS APIs (`dirs`, `walkdir`, `trash`, `libc`) rather than from a config file.

4. **Platform selection is auto-detected at runtime**, not configured: `src/main.ts` initializes `platformOs = "unknown"` and sets `diskLabel = "Macintosh HD"` as defaults, then switches UI text based on the detected platform. There is no user override.

5. **Capabilities are declarative**: `capabilities/default.json` lists the `main` window and grants `core:default` + `opener:default`. This is the only permission surface — no dynamic permission toggling.

6. **No secrets management**: No `.env`, no secret rotation, no encrypted config store. Sensitive operations (file deletion) go through Tauri commands and rely on OS-level permissions.

## Conventions Observed

- App identity is centralized in `tauri.conf.json` (`productName`, `version`, `identifier`) and mirrored in `Cargo.toml` and `package.json`.
- Frontend build output is always placed under `../dist` relative to `src-tauri`, as declared in `tauri.conf.json`'s `build.frontendDist`.
- Dev server port is fixed at `1420` across both `tauri.conf.json` and `vite.config.ts`; changing one without the other would break the dev workflow.
- Capability definitions live under `src-tauri/capabilities/` and reference the generated schema via `$schema: "../gen/schemas/desktop-schema.json"`.
- Platform-specific Rust features are expressed via `[target.'cfg(target_os = "...")'.dependencies]` blocks in `Cargo.toml`.

## Rules Enforced by Tooling

- `tauri.conf.json` is validated against the Tauri v2 schema (`$schema: https://schema.tauri.app/config/2`) by the Tauri CLI during build.
- `capabilities/default.json` is validated against the generated desktop schema (`$schema: ../gen/schemas/desktop-schema.json`).
- `vite.config.ts` enforces strict port binding (`strictPort: true`), so the dev server fails if port `1420` is already in use.
- `build.rs` delegates all Tauri build logic to `tauri_build::build()`, meaning any change to Tauri's build contract must go through that entry point.

## What Is Absent

There is no runtime configuration loader, no `.env` parsing, no YAML/TOML/JSON settings file read at startup, no feature flag system, and no mechanism for end users to modify application behavior after installation. Configuration is entirely static and baked into the binary.