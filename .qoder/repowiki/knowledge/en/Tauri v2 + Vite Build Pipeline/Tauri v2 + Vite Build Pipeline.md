---
kind: build_system
name: Tauri v2 + Vite Build Pipeline
category: build_system
scope:
    - '**'
source_files:
    - package.json
    - vite.config.ts
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/tauri.conf.json
---

## Build System Overview

This project is a Tauri v2 desktop application composed of two independently built layers:
- **Frontend**: TypeScript/Vite SPA, built by `vite build` and emitted to `dist/`.
- **Backend**: Rust crate (`freespace-macos`) compiled via Cargo/Tauri, which bundles the frontend into a native app bundle (`.app`, `.dmg`, Windows installer, etc.).

There is no Makefile, Dockerfile, CI pipeline, or release automation in the repository. The build is driven entirely through npm scripts and the Tauri CLI.

## Entry Points

- `package.json` defines four npm scripts: `dev` (`vite`), `build` (`tsc && vite build`), `preview` (`vite preview`), and `tauri` (`tauri`). There is no top-level script that invokes both frontend and backend; developers run `npm run tauri dev` / `npm run tauri build` from the repo root.
- `src-tauri/Cargo.toml` declares the Rust package (`freespace-macos`, edition 2021) with crate types `staticlib`, `cdylib`, `rlib`, and platform-scoped dependencies for macOS (`libc`) and Windows (`windows-sys`).
- `src-tauri/build.rs` is a thin shim that calls `tauri_build::build()`, delegating all Tauri-specific code generation to the Tauri build system.
- `src-tauri/tauri.conf.json` is the central orchestration file: it wires the frontend build into the Tauri lifecycle via `build.beforeDevCommand: "npm run dev"`, `build.devUrl: "http://localhost:1420"`, `build.beforeBuildCommand: "npm run build"`, and `build.frontendDist: "../dist"`.

## Frontend Build (Vite)

- `vite.config.ts` pins the dev server to port `1420` with `strictPort: true`, uses HMR over WebSocket on port `1421` when `TAURI_DEV_HOST` is set, and ignores `**/src-tauri/**` in watch mode so Rust changes do not trigger unnecessary frontend rebuilds.
- The production build runs `tsc` first (TypeScript compilation) then `vite build`, emitting assets under `dist/`.
- Dependencies are pinned via `package-lock.json`; versions use caret ranges (`^2`, `^6.0.3`, `~5.6.2`).

## Backend Build (Cargo + Tauri)

- `src-tauri/Cargo.toml` lists runtime deps: `tauri` v2, `tauri-plugin-opener`, `serde`/`serde_json`, `walkdir`, `trash`, `dirs`, plus OS-specific crates.
- `src-tauri/tauri.conf.json` sets `bundle.active = true`, `bundle.targets = "all"`, and enumerates icon assets across platforms (`icons/icon.icns`, `icons/icon.ico`, PNG variants). The generated schema files under `src-tauri/gen/schemas/` are produced by Tauri's codegen.
- Versioning is declared in two places: `package.json` (`version: "0.1.0"`) and `src-tauri/Cargo.toml` (`version = "0.1.0"`); `tauri.conf.json` also has its own `version: "0.1.0"`. These three values are kept in sync manually — there is no single source of truth enforced by the build.

## Icon Generation

- `generate_icon.py` is a standalone Python script (PIL + NumPy) that procedurally renders a 1024×1024 macOS squircle icon with gradients, glow effects, and a "magic broom" motif. It writes directly to an absolute path (`/Volumes/SSD.DATA/Leo/APPS/FreeSpaceMacos/app-icon-1024.png`), so it is not part of the automated build — it is a one-off asset generator.
- The resulting `app-icon-1024.png` lives at the repo root, while the per-platform icons under `src-tauri/icons/` (macOS `.icns`, Windows `.ico`, Android mipmap dirs, iOS AppIcon set, Windows Store logos) are committed as pre-generated artifacts consumed by `tauri.conf.json`'s `bundle.icon` list.

## Output Artifacts

Per the README, a release build produces:
- `src-tauri/target/release/bundle/macos/FreeSpace.app`
- `src-tauri/target/release/bundle/dmg/FreeSpace_0.1.0_aarch64.dmg`

These are the standard Tauri output locations; no post-processing or signing step is configured in this repo.

## Conventions & Constraints

- The frontend and backend live in sibling directories (`src/` and `src-tauri/`), a convention enforced by `tauri.conf.json`'s `frontendDist: "../dist"`.
- Tauri's dev server is hard-coded to port `1420` and HMR to port `1421` in `vite.config.ts`; changing these requires updating both the Vite config and `tauri.conf.json`'s `devUrl`.
- Platform-specific Rust features are gated via `[target.'cfg(target_os = "...")'.dependencies]` blocks in `Cargo.toml` rather than feature flags.
- No CI, Docker, Makefile, or release automation exists in the repository — builds are local-only.
- Version numbers must be updated in three files (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`) manually; there is no tooling enforcing consistency.