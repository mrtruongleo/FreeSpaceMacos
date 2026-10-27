# Data Models and Structures

<cite>
**Referenced Files in This Document**
- [lib.rs](file://src-tauri/src/lib.rs)
- [main.ts](file://src/main.ts)
- [README.md](file://README.md)
</cite>

## Update Summary
**Changes Made**
- Added documentation for the new LargeFile struct and its role in the large files scanning feature
- Updated Core Components section to include LargeFile
- Added new LargeFiles Scanning Feature section with detailed analysis
- Updated Architecture Overview to include scan_large_files command
- Enhanced Detailed Component Analysis with LargeFile structure details
- Updated diagrams to reflect the new data model relationships

## Table of Contents
1. [Introduction](#introduction)
2. [Project Structure](#project-structure)
3. [Core Components](#core-components)
4. [Architecture Overview](#architecture-overview)
5. [Detailed Component Analysis](#detailed-component-analysis)
6. [LargeFiles Scanning Feature](#largefiles-scanning-feature)
7. [Dependency Analysis](#dependency-analysis)
8. [Performance Considerations](#performance-considerations)
9. [Troubleshooting Guide](#troubleshooting-guide)
10. [Conclusion](#conclusion)

## Introduction
This document explains the Rust data models used by the FreeSpaceMacos backend to represent a hierarchical cleanup system. It focuses on CleanupItem, SubCleanupItem, ScanResult, DiskUsage, SafetyLevel, DeleteResult, and the new LargeFile struct. The documentation covers field purposes, serde serialization behavior, relationships between models, and how these structures support UI rendering and user workflows. It also includes examples of model instantiation patterns, JSON serialization/deserialization expectations, and usage flows across Tauri commands.

The project is a macOS storage cleanup app built with Tauri 2 (Rust + TypeScript/HTML5/CSS). The backend scans directories, computes disk usage, categorizes items by safety level, supports deletion operations, and provides specialized scanning for large individual files. The frontend mirrors the Rust types as TypeScript interfaces for type-safe communication.

**Section sources**
- [README.md:1-74](file://README.md#L1-L74)

## Project Structure
At a high level, the Rust backend defines data models and Tauri commands in lib.rs. The frontend main.ts declares matching TypeScript interfaces that mirror the Rust types for cross-process communication.

```mermaid
graph TB
subgraph "Frontend"
FE_TS["TypeScript Interfaces<br/>main.ts"]
end
subgraph "Backend (Tauri)"
BE_LIB["Rust Types & Commands<br/>src-tauri/src/lib.rs"]
end
FE_TS --> |"invoke() calls"| BE_LIB
```

**Diagram sources**
- [lib.rs:1-74](file://src-tauri/src/lib.rs#L1-L74)
- [main.ts:3-63](file://src/main.ts#L3-L63)

**Section sources**
- [lib.rs:1-74](file://src-tauri/src/lib.rs#L1-L74)
- [main.ts:3-63](file://src/main.ts#L3-L63)

## Core Components
This section summarizes the core data models and their roles.

- SafetyLevel: Enumerates whether an item can be safely deleted or needs user review before deletion.
- SubCleanupItem: Represents a child entry within a directory during scanning; supports optional nested children for deeper exploration.
- CleanupItem: Represents a top-level scan target with metadata, safety classification, and immediate children.
- DiskUsage: Aggregates total, used, free space and percentage used.
- ScanResult: Wraps disk usage plus categorized lists of safe and review items, along with totals.
- DeleteResult: Reports deletion outcomes including success flag, counts, freed bytes, and errors.
- LargeFile: Represents individual large files found during scanning, including name, path, and size properties for the new large files scanning feature.

These models are serialized via serde and exposed through Tauri commands to the frontend.

**Section sources**
- [lib.rs:8-74](file://src-tauri/src/lib.rs#L8-L74)

## Architecture Overview
The backend exposes several Tauri commands that operate on the data models:

- get_disk_usage: Returns DiskUsage.
- scan_system: Returns ScanResult populated with CleanupItem lists and totals.
- scan_sub_items: Returns Vec<SubCleanupItem> for a given path.
- scan_large_files: Returns Vec<LargeFile> for large file scanning functionality.
- delete_items: Accepts paths and returns DeleteResult.

```mermaid
sequenceDiagram
participant FE as "Frontend (TypeScript)"
participant Tauri as "Tauri Runtime"
participant Lib as "Rust Backend (lib.rs)"
FE->>Tauri : invoke("scan_system")
Tauri->>Lib : scan_system()
Lib-->>Tauri : ScanResult
Tauri-->>FE : ScanResult
FE->>Tauri : invoke("scan_sub_items", path)
Tauri->>Lib : scan_sub_items(path)
Lib-->>Tauri : Vec<SubCleanupItem>
Tauri-->>FE : Vec<SubCleanupItem>
FE->>Tauri : invoke("scan_large_files", minSizeMb, limit)
Tauri->>Lib : scan_large_files(minSizeMb, limit)
Lib-->>Tauri : Vec<LargeFile>
Tauri-->>FE : Vec<LargeFile>
FE->>Tauri : invoke("delete_items", paths[])
Tauri->>Lib : delete_items(paths)
Lib-->>Tauri : DeleteResult
Tauri-->>FE : DeleteResult
```

**Diagram sources**
- [lib.rs:335-406](file://src-tauri/src/lib.rs#L335-L406)
- [lib.rs:408-415](file://src-tauri/src/lib.rs#L408-L415)
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)
- [lib.rs:1164-1190](file://src-tauri/src/lib.rs#L1164-L1190)

## Detailed Component Analysis

### SafetyLevel
Purpose:
- Classifies items into Safe or NeedsReview to guide UI behavior and deletion confirmation flows.

Serialization:
- Serialized as string variants ("Safe", "NeedsReview").

Relationships:
- Used by CleanupItem to indicate deletion policy.

Design notes:
- Simple enum keeps UI logic straightforward: auto-delete Safe items after confirmation; require explicit review for NeedsReview items.

**Section sources**
- [lib.rs:8-12](file://src-tauri/src/lib.rs#L8-L12)

### SubCleanupItem
Purpose:
- Represents a single directory entry discovered during scanning, including size, file count, directory flags, and optional nested children.

Key fields:
- id: Unique identifier for the entry (typically its path).
- name: Display name.
- path: Filesystem path.
- size: Total size of the entry (for directories, aggregated).
- file_count: Number of files counted.
- is_dir: Whether the entry is a directory.
- has_children: Hint that the directory contains more entries (used for lazy loading).
- children: Optional nested list for deeper traversal.

Serialization:
- children is omitted when None using skip_serializing_if to reduce payload size.

Relationships:
- Child of CleanupItem.children at the first level; can contain further SubCleanupItem entries for deeper levels.

Design notes:
- Lazy expansion: children starts as None and can be populated on demand via scan_sub_items.

**Section sources**
- [lib.rs:14-25](file://src-tauri/src/lib.rs#L14-L25)
- [lib.rs:115-168](file://src-tauri/src/lib.rs#L115-L168)

### CleanupItem
Purpose:
- Represents a top-level cleanup category with metadata and immediate children.

Key fields:
- id: Stable identifier for the category.
- name: Human-readable label.
- path: Root path being scanned.
- size: Aggregated size of the root path.
- category: Grouping label (e.g., Caches, Developer, Browser).
- icon: UI icon representation.
- safety_level: Safety classification.
- description: Explanation of what the category contains.
- safety_tip: Guidance for users about risks and recovery.
- file_count: Number of files under the root path.
- exists: Whether the path currently exists.
- children: Immediate SubCleanupItem entries.

Serialization:
- All fields are included; children always present as a vector (empty if none).

Relationships:
- Contains SubCleanupItem entries.
- Part of ScanResult.safe_items or ScanResult.review_items.

Design notes:
- Exists and size enable UI filtering to show only relevant categories.
- Safety tip and description inform user decisions.

**Section sources**
- [lib.rs:27-41](file://src-tauri/src/lib.rs#L27-L41)
- [lib.rs:170-208](file://src-tauri/src/lib.rs#L170-L208)

### DiskUsage
Purpose:
- Provides current disk statistics for display in the UI gauge.

Key fields:
- total: Total capacity.
- used: Space used.
- free: Available space.
- percentage_used: Percentage of used space.

Serialization:
- Numeric values serialized directly.

Usage:
- Returned by get_disk_usage and embedded in ScanResult.

**Section sources**
- [lib.rs:43-49](file://src-tauri/src/lib.rs#L43-L49)
- [lib.rs:335-406](file://src-tauri/src/lib.rs#L335-L406)

### ScanResult
Purpose:
- Aggregates scan outputs: disk usage, categorized cleanup items, and totals.

Key fields:
- disk_usage: Current disk stats.
- safe_items: Categories eligible for automatic deletion.
- review_items: Categories requiring user review.
- total_safe_size: Sum of sizes in safe_items.
- total_review_size: Sum of sizes in review_items.

Serialization:
- All fields included.

Usage:
- Returned by scan_system; drives UI sections for safe and review categories.

**Section sources**
- [lib.rs:51-58](file://src-tauri/src/lib.rs#L51-L58)
- [lib.rs:518-1162](file://src-tauri/src/lib.rs#L518-L1162)

### DeleteResult
Purpose:
- Reports the outcome of deletion operations.

Key fields:
- success: True if no errors occurred.
- deleted_count: Number of items/files removed.
- freed_bytes: Total bytes freed.
- errors: List of error messages encountered.

Serialization:
- All fields included.

Usage:
- Returned by delete_items; informs UI feedback and progress.

**Section sources**
- [lib.rs:60-66](file://src-tauri/src/lib.rs#L60-L66)
- [lib.rs:1164-1190](file://src-tauri/src/lib.rs#L1164-L1190)

### LargeFile
Purpose:
- Represents individual large files discovered during the large files scanning feature, providing essential information for users to identify and manage large files consuming disk space.

Key fields:
- name: The filename of the large file.
- path: Complete filesystem path to the file.
- size: Size of the file in bytes.

Serialization:
- All fields are included with standard serde serialization.

Relationships:
- Used as return type for scan_large_files command.
- Mirrored by TypeScript interface for frontend consumption.

Design notes:
- Simple structure optimized for efficient scanning and transmission of large file information.
- Supports filtering by minimum size threshold and result limiting for performance.
- Enables direct file reveal functionality through the existing reveal_path command.

**Section sources**
- [lib.rs:68-73](file://src-tauri/src/lib.rs#L68-L73)
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)
- [main.ts:59-63](file://src/main.ts#L59-L63)

### Hierarchical Cleanup System
The hierarchy is represented as:
- ScanResult contains two lists of CleanupItem (safe and review).
- Each CleanupItem has a children vector of SubCleanupItem.
- SubCleanupItem may have optional nested children for deeper exploration.
- LargeFile operates independently as a flat list of large files found across the filesystem.

```mermaid
classDiagram
class ScanResult {
+disk_usage : DiskUsage
+safe_items : Vec<CleanupItem>
+review_items : Vec<CleanupItem>
+total_safe_size : u64
+total_review_size : u64
}
class CleanupItem {
+id : String
+name : String
+path : String
+size : u64
+category : String
+icon : String
+safety_level : SafetyLevel
+description : String
+safety_tip : String
+file_count : u64
+exists : bool
+children : Vec<SubCleanupItem>
}
class SubCleanupItem {
+id : String
+name : String
+path : String
+size : u64
+file_count : u64
+is_dir : bool
+has_children : bool
+children? : Option<Vec<SubCleanupItem>>
}
class LargeFile {
+name : String
+path : String
+size : u64
}
class DiskUsage {
+total : u64
+used : u64
+free : u64
+percentage_used : f64
}
class SafetyLevel {
<<enum>>
+Safe
+NeedsReview
}
ScanResult --> DiskUsage : "contains"
ScanResult --> CleanupItem : "contains multiple"
CleanupItem --> SubCleanupItem : "contains multiple"
CleanupItem --> SafetyLevel : "uses"
ScanResult --> LargeFile : "independent from"
```

**Diagram sources**
- [lib.rs:8-74](file://src-tauri/src/lib.rs#L8-L74)

## LargeFiles Scanning Feature
The large files scanning feature provides a specialized way to identify and manage individual large files across the user's home directory. This complements the existing cleanup system by focusing on specific large files rather than directory-based cleanup categories.

### Implementation Details
The `scan_large_files` function performs a comprehensive scan of the home directory with the following characteristics:

- **Depth Limit**: Scans up to 24 levels deep to capture deeply nested large files while preventing excessive recursion.
- **Filtering Logic**: Excludes hidden files/directories (starting with '.') and system directories like "$RECYCLE.BIN" and "System Volume Information".
- **Size Filtering**: Only includes files larger than the specified minimum size threshold (converted from MB to bytes).
- **Result Limiting**: Caps results to prevent overwhelming the UI with too many files (default limit of 200).
- **Sorting**: Results are sorted by size in descending order to prioritize the largest files.

### Usage Pattern
The frontend integrates this feature through:

1. **User Interface**: A dedicated tab for large file scanning with configurable minimum size thresholds.
2. **Command Invocation**: Calls `scan_large_files` with parameters for minimum size (in MB) and result limit.
3. **Display**: Renders results in a ranked list showing file name, directory path, size, and reveal functionality.
4. **Integration**: Works alongside the existing cleanup system without interfering with directory-based operations.

### Performance Considerations
- Uses WalkDir with depth limits to prevent excessive filesystem traversal.
- Implements early filtering to skip unnecessary directories and files.
- Limits result set size to maintain responsive UI performance.
- Leverages existing file icon detection for consistent visual presentation.

**Section sources**
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)
- [main.ts:1112-1182](file://src/main.ts#L1112-L1182)

## Dependency Analysis
The data models depend on:
- serde for serialization/deserialization.
- std::fs and walkdir for directory traversal and size computation.
- Platform-specific crates (libc, windows_sys) for disk usage queries.
- dirs crate for home directory resolution in large file scanning.

```mermaid
graph LR
Serde["serde"] --> Models["Data Models"]
StdFS["std::fs"] --> Helpers["Helpers"]
WalkDir["walkdir"] --> Helpers
Dirs["dirs"] --> LargeFileCmd["scan_large_files"]
Libc["libc (macOS)"] --> DiskUsageCmd["get_disk_usage"]
WinSys["windows_sys (Windows)"] --> DiskUsageCmd
Helpers --> Models
Models --> Commands["Tauri Commands"]
```

**Diagram sources**
- [lib.rs:1-4](file://src-tauri/src/lib.rs#L1-L4)
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)
- [lib.rs:335-406](file://src-tauri/src/lib.rs#L335-L406)

**Section sources**
- [lib.rs:1-4](file://src-tauri/src/lib.rs#L1-L4)
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)

## Performance Considerations
- Directory size calculation uses a depth limit to avoid excessive recursion and improve performance.
- Children scanning sorts results by size to prioritize larger items in the UI.
- Skipping empty entries reduces noise and improves responsiveness.
- Using skip_serializing_if for optional children reduces JSON payload size when not expanded.
- Large file scanning implements multiple optimization strategies including depth limits, filtering, and result capping.

Recommendations:
- For very large directories, consider pagination or virtualization in the UI.
- Cache computed sizes where appropriate to avoid repeated scans.
- Use the large files feature for targeted file management alongside the directory-based cleanup system.

[No sources needed since this section provides general guidance]

## Troubleshooting Guide
Common issues and resolutions:
- Path resolution: Ensure home directory expansion works correctly; invalid or non-existent paths result in zero-sized items and empty children.
- Permission errors: Deletion may fail due to permissions; check DeleteResult.errors for details.
- Platform differences: Disk usage APIs differ by OS; ensure correct platform detection and fallbacks.
- Large file scanning performance: Very large filesystems may take time to scan; use appropriate minimum size thresholds to balance thoroughness with speed.

Operational tips:
- Use scan_sub_items to lazily expand directories and inspect contents before deletion.
- Validate existence and size before presenting items to users.
- Leverage the large files feature for identifying oversized individual files that may not fit into cleanup categories.
- Combine both approaches: use large file scanning for specific large files and directory cleanup for systematic cache/log removal.

**Section sources**
- [lib.rs:115-168](file://src-tauri/src/lib.rs#L115-L168)
- [lib.rs:1164-1190](file://src-tauri/src/lib.rs#L1164-L1190)
- [lib.rs:438-481](file://src-tauri/src/lib.rs#L438-L481)

## Conclusion
The FreeSpaceMacos backend models provide a clear, serializable representation of the cleanup system. SafetyLevel guides user interactions, while CleanupItem and SubCleanupItem form a flexible hierarchy supporting both flat and nested views. ScanResult aggregates scan outputs for UI consumption, and DeleteResult offers robust feedback for deletion operations. The new LargeFile struct extends this capability by providing specialized scanning for individual large files, complementing the directory-based cleanup approach. These design choices align with UI requirements for clarity, safety, and performance, enabling efficient scanning and controlled deletion across platforms while offering multiple strategies for reclaiming disk space.

[No sources needed since this section summarizes without analyzing specific files]