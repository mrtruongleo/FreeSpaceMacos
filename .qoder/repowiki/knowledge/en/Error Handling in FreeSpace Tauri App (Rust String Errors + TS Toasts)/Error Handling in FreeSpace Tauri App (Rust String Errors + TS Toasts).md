---
kind: error_handling
name: Error Handling in FreeSpace Tauri App (Rust String Errors + TS Toasts)
category: error_handling
scope:
    - '**'
source_files:
    - src-tauri/src/lib.rs
    - src-tauri/src/main.rs
    - src/main.ts
---

## Overview

The FreeSpace macOS desktop app is a small Tauri v2 application with a Rust backend (`src-tauri/src/lib.rs`) and a TypeScript/Vite frontend (`src/main.ts`). Error handling is informal and localized — there is no centralized error type, no custom `std::error::Error` hierarchy, no panic/recover strategy, and no middleware. Errors flow across the IPC boundary as plain strings.

## Rust Backend (`src-tauri/src/lib.rs`)

### Error return convention

All `#[tauri::command]` functions that can fail return `Result<T, String>`:

- `get_disk_usage() -> Result<DiskUsage, String>`
- `scan_sub_items(path: String) -> Result<Vec<SubCleanupItem>, String>`
- `get_tree_root() -> Result<SubCleanupItem, String>`
- `scan_large_files(min_size_mb: u64, limit: usize) -> Result<Vec<LargeFile>, String>`
- `reveal_path(path: String) -> Result<(), String>`
- `delete_items(paths: Vec<String>) -> Result<DeleteResult, String>`

Errors are propagated using idiomatic Rust combinators on `Option`/`Result`:

- `.ok_or_else(|| "Could not resolve home directory".to_string())?` — converts an `Option<PathBuf>` into a `String` error when the home directory cannot be resolved.
- `.map_err(|e| e.to_string())?` — used for `CString::new(...)` and `Command::spawn(...)` to turn OS-level errors into user-facing strings.
- `match fs::read_dir(path) { Ok(e) => e, Err(e) => return (0, 0, Some(e.to_string())) }` — early-return with an error string from helper functions.

There is no dedicated error enum or module; every error is an ad-hoc `String` literal or a formatted message such as `format!("Could not remove file: {}", path_str)`.

### Partial-failure aggregation via `DeleteResult`

The delete operation deliberately does **not** fail fast. `delete_target` returns `(u64, u32, Option<String>)` — freed bytes, deleted count, and an optional last error. `delete_items` iterates all requested paths, accumulates partial results, and collects per-path error messages into `Vec<String>`. The final `DeleteResult` struct carries both success metrics and an `errors: Vec<String>` field so the frontend can report partial failures without aborting the whole batch.

### Silent fallbacks (non-error paths)

Several helpers treat missing data as empty rather than errors:

- `get_dir_size`: if `!path.exists()` it returns `(0, 0)`.
- `scan_children_direct`: if `!path.exists() || !path.is_dir()` it returns `Vec::new()`; `fs::read_dir` errors are swallowed via `filter_map(|e| e.ok())`.
- `expand_home`: if `dirs::home_dir()` returns `None`, the input path is used verbatim.
- `get_disk_usage` falls back to zeroed `DiskUsage` inside `scan_system` via `.unwrap_or(DiskUsage { ... })`.

### Platform-specific error paths

`reveal_path` uses `#[cfg(target_os = "macos")]` / `#[cfg(target_os = "windows")]` / `#[cfg(not(any(...)))]` branches. On unsupported platforms it returns `Err("Reveal is not supported on this platform".to_string())`.

### Application entry point

`main.rs` delegates to `freespace_lib::run()`, which builds the Tauri app and calls `.expect("error while running tauri application")` on the result of `.run(...)`. There is no `panic!` or `catch_unwind` anywhere in the codebase — panics are treated as unrecoverable crashes.

## TypeScript Frontend (`src/main.ts`)

### Invocation error handling

Every call to `invoke<T>(...)` is wrapped in `try/catch` blocks. Errors thrown by the Rust backend (serialized as the `String` error payload) surface as the caught value and are handled uniformly:

```ts
try {
  const result = await invoke<ScanResult>("scan_system");
  // ...
} catch (err) {
  console.error("Scan error:", err);
  showToast(`Scan failed: ${err}`, "error");
}
```

The same pattern is used for `runClean`, `toggleNodeExpand` (sub-item loading), and other async operations.

### User-facing error presentation

Errors are surfaced through a local `showToast(message, type)` function (lines 136–155) that renders a transient toast with three types: `"success" | "error" | "info"`. There is no global error boundary, no logging framework, and no structured error object — the raw error string is interpolated directly into the toast message.

### Partial-delete UI

For `delete_items`, the frontend inspects the returned `DeleteResult.success` and `errors` fields:

```ts
if (result.success || result.freed_bytes > 0) {
  showToast(`Cleanup successful! Freed ${formatBytes(result.freed_bytes)}...`, "success");
} else if (result.errors.length > 0) {
  showToast(`Finished with issues: ${result.errors[0]}`, "error");
}
```

Only the first error is shown to the user; the rest are silently dropped.

## Conventions and Constraints

1. **Backend errors are plain `String`s.** All Tauri commands use `Result<T, String>`; there is no custom error type, no error codes, and no `thiserror`/`anyhow` dependency observed in the code.
2. **Errors cross the IPC boundary as strings.** The Rust `String` error payload is serialized and re-thrown in TypeScript as the caught exception value.
3. **Delete operations aggregate partial failures instead of failing fast.** Individual path deletions do not abort the batch; errors are collected in `DeleteResult.errors` and reported to the caller.
4. **Missing or unreadable directories are treated as empty results, not errors.** Helpers like `get_dir_size`, `scan_children_direct`, and `expand_home` return defaults (`(0,0)`, `Vec::new()`, or the raw path) rather than propagating errors.
5. **Frontend catches every `invoke` call individually.** There is no shared error interceptor; each async action has its own `try/catch` block that logs via `console.error` and shows a toast.
6. **No `panic!`, `assert!`, or `unreachable!` is used for control flow.** The only `.expect(...)` is on the Tauri runtime's `.run()` result at process startup.
7. **No middleware or global error handler exists.** Error handling is inline at the call site in both Rust and TypeScript.
8. **User-visible error messages are hand-written English strings** embedded directly in command implementations (e.g., `"Path does not exist"`, `"Could not get disk usage"`, `"Could not resolve home directory"`).