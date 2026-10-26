---
kind: business_term
name: Business Glossary
category: business_term
scope:
    - '**'
---

### Dọn dẹp
- Definition：Vietnamese tab label meaning 'Cleanup'; the primary tab showing the disk usage gauge and the list of safe-to-clean items (user caches, logs, crash reports, trash, browser caches) with direct removal.
- Aliases：cleanup tab

### Cây thư mục
- Definition：Vietnamese tab label meaning 'Directory Tree'; shows the user's home folder as a size-sorted tree (largest first) with lazy expansion of children and a relative size bar per row.
- Aliases：tree tab、size tree

### Tệp lớn
- Definition：Vietnamese tab label meaning 'Large Files'; scans the home directory for individual files above a configurable threshold (50 MB–1 GB) and lists them ranked by size with a Reveal button.
- Aliases：large files tab、files tab

### Reveal
- Definition：Action that opens the selected file in the native file manager: `explorer /select,<path>` on Windows (Explorer) and `open -R <path>` on macOS (Finder). Implemented as a dedicated Tauri command rather than relying on the opener plugin's `reveal_item_in_dir`.
- Aliases：reveal item、show in explorer、show in finder

### Windows Cleaner
- Definition：Badge text shown on Windows indicating the app has auto-detected the platform and is scanning Windows-specific cache/temp locations (AppData/Local/Temp, Chrome/Edge caches, Recycle Bin, Windows.old, npm/pip/uv/cargo caches, etc.).
- Aliases：badge: Windows Cleaner

### Macintosh HD
- Definition：Hardcoded disk label displayed on macOS in the UI; replaced by `Local Disk (C:)` on Windows after the auto-detection change.
- Aliases：disk label: Macintosh HD
