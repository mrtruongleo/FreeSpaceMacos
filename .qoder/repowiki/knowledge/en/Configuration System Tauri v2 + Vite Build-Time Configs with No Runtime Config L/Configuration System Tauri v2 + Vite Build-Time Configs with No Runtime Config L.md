---
kind: configuration_system
name: 'Configuration System: Tauri v2 + Vite Build-Time Configs with No Runtime Config Layer'
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
    - src-tauri/src/main.rs
---

## Overview

This FreeSpace Desktop App (Tauri v2 + Vite) has **no runtime configuration system**. All application behavior is configured at **build/compile time** via three static JSON/TOML/TS config files plus a few environment variables consumed only by the dev server. There is no `.env`, no YAML/TOML user config loader, no feature-flag registry, and no secrets manager.

## Key Configuration Files

### `src-tauri/tauri.conf.json` — Tauri app manifest
- Declares `productName`, `version`, `identifier`, window geometry (`width`, `height`, `minWidth`, `minHeight`, `center`, `resizable`, `titleBarStyle`, `hiddenTitle`, `decorations`, `transparent`), CSP (`null`), and bundle icons.
- `build.beforeDevCommand = "npm run dev"`, `build.devUrl = "http://localhost:1420"`, `build.beforeBuildCommand = "npm run build"`, `build.frontendDist = "../dist"` — wires Vite into the Tauri dev/build pipeline.
- `app.withGlobalTauri = true` exposes the global `window.__TAURI__` API to the frontend.
- `bundle.targets = "all"` bundles for all supported platforms.

### `vite.config.ts` — Vite dev-server config
- Reads `process.env.TAURI_DEV_HOST` to configure HMR host/port (`ws` protocol, port `1421`).
- Forces `server.port = 1420`, `server.strictPort = true`, and ignores `**/src-tauri/**` from watching.
- This is the only place an environment variable is consumed; it affects only the dev server, not the shipped app.

### `package.json` — NPM entrypoint & scripts
- Scripts: `dev` → `vite`, `build` → `tsc && vite build`, `preview` → `vite preview`, `tauri` → `tauri`.
- Dependencies pin `@tauri-apps/api ^2`, `@tauri-apps/plugin-opener ^2`, `@tauri-apps/cli ^2`, `vite ^6.0.3`, `typescript ~5.6.2`.

### `src-tauri/Cargo.toml` — Rust crate metadata & conditional deps
- Crate name `freespace-macos`, lib name `freespace_lib`, edition `2021`, crate-types `["staticlib", "cdylib", "rlib"]`.
- Conditional dependencies via `[target.'cfg(target_os = "macos")'.dependencies]` (`libc`) and `[target.'cfg(target_os = "windows")'.dependencies]` (`windows-sys` with `Win32_Storage_FileSystem` feature).
- Core deps: `tauri 2`, `tauri-plugin-opener 2`, `serde` (derive), `serde_json`, `walkdir 2`, `trash 5`, `dirs 6`.

### `src-tauri/build.rs` — Tauri build hook
- Single line: `tauri_build::build()` — delegates all Tauri codegen/schema generation to the Tauri build helper.

### `src-tauri/capabilities/default.json` — Tauri capability policy
- Capability identifier `default`, applies to window `main`, grants permissions `core:default` and `opener:default`. Generated schemas live under `src-tauri/gen/schemas/`.

## Architecture & Conventions

1. **Config-as-code**: Application behavior (scan targets, safety levels, descriptions, icons) is hardcoded as Rust structs and vectors inside `src-tauri/src/lib.rs` — there is no external config file that can be edited by users or CI.
2. **Platform branching via `#[cfg(target_os = ...)]`**: Separate `scan_system()` implementations exist for macOS and Windows, selected at compile time. Paths like `~/Library/Caches` vs `~/AppData/Local/Temp` are branch-specific constants.
3. **Path resolution convention**: User paths use the `~` prefix, expanded via a local `expand_home()` helper that calls `dirs::home_dir()` — no shell-style env-var path expansion.
4. **Environment variables are dev-only**: `TAURI_DEV_HOST` is read in `vite.config.ts` solely to wire HMR during `tauri dev`; it is not compiled into the release binary.
5. **No runtime config loader**: The Rust `main.rs` is a thin shim that just calls `freespace_lib::run()`. No `dotenv`, no `config` crate, no CLI args parsing for config overrides.
6. **Security model**: CSP is disabled (`"csp": null`) and the default capability grants only `core:default` + `opener:default` — no filesystem or IPC permissions beyond what the opener plugin needs.
7. **Versioning is duplicated**: `version = "0.1.0"` appears in both `package.json` and `src-tauri/Cargo.toml` (and also in `tauri.conf.json`); there is no single source of truth script keeping them in sync.

## Conventions Observed

- Tauri app identity lives in `src-tauri/tauri.conf.json` (`productName`, `identifier`, `version`).
- Frontend build is driven by Vite, invoked from Tauri via `beforeDevCommand` / `beforeBuildCommand`.
- Platform-specific behavior is expressed with Rust `#[cfg(target_os = "...")]` blocks rather than runtime OS detection.
- User-facing scan categories are defined inline as `CleanupItem` literals in `lib.rs` with fields `id`, `name`, `path`, `category`, `icon`, `safety_level`, `description`, `safety_tip`.
- Permission surface is declared declaratively in `capabilities/default.json` and auto-generated schema references (`$schema` pointing at `../gen/schemas/desktop-schema.json`).

## Rules Enforced by the Code

- The dev server must listen on port `1420` with `strictPort: true` (enforced by `vite.config.ts`).
- HMR WebSocket connects to port `1421` when `TAURI_DEV_HOST` is set (enforced by the same file).
- The Tauri window label used by capabilities is `main` (enforced by matching the `label` field in `tauri.conf.json` and the `windows: ["main"]` entry in `capabilities/default.json`).
- Only the `core:default` and `opener:default` Tauri permissions are granted to the `main` window (enforced by `capabilities/default.json`).
- Release builds on Windows suppress the console subsystem via `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` in `src-tauri/src/main.rs`.