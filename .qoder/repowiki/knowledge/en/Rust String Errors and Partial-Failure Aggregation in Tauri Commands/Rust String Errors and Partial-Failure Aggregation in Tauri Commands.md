---
kind: error_handling
name: Rust String Errors and Partial-Failure Aggregation in Tauri Commands
category: error_handling
scope:
    - '**'
source_files:
    - src-tauri/src/lib.rs
    - src-tauri/src/main.rs
    - src-tauri/Cargo.toml
---

# Error Handling in FreeSpace Desktop App

## Approach Overview

The Rust backend (`src-tauri/src/lib.rs`) uses a minimal, idiomatic-Rust error strategy with no dedicated error types or external error-handling crate:

- **Tauri command return type**: Every `#[tauri::command]` returns `Result<T, String>` — the error variant is a plain `String` message. There are no custom error enums, no `thiserror`/`anyhow`, and no error codes.
- **Internal helpers** (e.g. `delete_target`, `delete_dir_contents_resilient`) do not return `Result`; instead they return a tuple `(u64, u32, Option<String>)` where the third element is an optional human-readable error string. This lets bulk operations continue past individual failures and aggregate them.
- **Frontend-facing partial failure**: The `DeleteResult` struct (lines 60–66) carries both success metrics (`success: bool`, `deleted_count`, `freed_bytes`) and an `errors: Vec<String>` of per-path messages. `success` is derived as `errors.is_empty()` (line 1185), so callers can inspect partial results even when some paths failed.
- **I/O errors are swallowed at the scan level**: `get_dir_size`, `scan_children_direct`, and `scan_large_files` use `.map(|m| m.len()).unwrap_or(0)` / `.filter_map(|e| e.ok())` to treat unreadable files/dirs as zero-size or skip them silently. Only path-level validation errors (missing home dir, non-existent path) bubble up as `String` errors via `?`.
- **Platform-specific fallbacks**: `get_disk_usage` returns `Err("Could not get disk usage".to_string())` when neither the macOS nor Windows branch succeeds; `reveal_path` has a `#[cfg(not(any(target_os = "macos", target_os = "windows")))]` branch returning `Err("Reveal is not supported on this platform".to_string())`.
- **No panic/recover**: No `panic!`, `assert!`, or `expect!` appears in business logic. The only `expect` is in the app entry point (`run()`, line 1209): `.expect("error while running tauri application")`, which crashes the process if Tauri's runtime fails to start — appropriate for a fatal bootstrap error.
- **No middleware**: There is no global error handler, logging layer, or Tauri error interceptor. Errors propagate directly from commands to the TypeScript frontend as the second argument of the promise rejection.

## Key Files

- `src-tauri/src/lib.rs` — all Tauri commands, data models, and error-producing logic.
- `src-tauri/src/main.rs` — thin entry that delegates to `freespace_lib::run()`.
- `src-tauri/Cargo.toml` — confirms no error-handling dependencies beyond `serde`/`tauri`.

## Conventions Observed

1. **Every public Tauri command returns `Result<T, String>`**. The error payload is always a user-facing English/Vietnamese string literal (e.g. `"Path does not exist"`, `"Could not resolve home directory"`).
2. **Internal I/O is resilient by default**: functions like `get_dir_size` and `scan_children_direct` never fail outwardly; missing or unreadable entries are treated as empty/zero-size rather than propagated.
3. **Destructive operations aggregate errors**: `delete_items` iterates over all requested paths, collects per-path error strings into `DeleteResult.errors`, and sets `success` based on whether any occurred. Callers should check both `success` and `errors`.
4. **Validation-only errors surface to the caller**: `get_tree_root`, `scan_large_files`, and `reveal_path` use `ok_or_else`/early-return `Err(...)` for precondition failures (missing home dir, nonexistent path).
5. **Platform branches return explicit fallbacks**: unsupported platforms receive descriptive `Err` strings rather than panicking or returning defaults.
6. **No structured error taxonomy**: there is no enum distinguishing permission errors, IO errors, network errors, etc. All errors are flat strings.