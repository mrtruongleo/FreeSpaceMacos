# Frontend Architecture (TypeScript)

<cite>
**Referenced Files in This Document**
- [README.md](file://README.md)
- [package.json](file://package.json)
- [index.html](file://index.html)
- [src/main.ts](file://src/main.ts)
- [src/styles.css](file://src/styles.css)
</cite>

## Table of Contents
1. [Introduction](#introduction)
2. [Project Structure](#project-structure)
3. [Core Components](#core-components)
4. [Architecture Overview](#architecture-overview)
5. [Detailed Component Analysis](#detailed-component-analysis)
6. [Dependency Analysis](#dependency-analysis)
7. [Performance Considerations](#performance-considerations)
8. [Troubleshooting Guide](#troubleshooting-guide)
9. [Conclusion](#conclusion)

## Introduction
This document explains the frontend architecture of FreeSpaceMacos, a macOS storage cleanup and disk analyzer built with Tauri 2, TypeScript, HTML5, and CSS. The application scans system caches and developer artifacts, presents results in a hierarchical tree, supports safe and review categories, and performs deletions via Tauri IPC. It uses glassmorphism styling, responsive design, toast notifications, and confirmation dialogs to provide clear user feedback.

Key goals:
- Explain state management for hierarchical tree navigation and parent-child selection logic.
- Detail UI rendering components including dynamic tree generation, confirmation dialogs, and toast notifications.
- Document Tauri command integration patterns for IPC between frontend and backend.
- Describe glassmorphism CSS styling, responsive design, and visual feedback mechanisms.
- Provide examples of event handling, state synchronization, and component composition through code paths.
- Address user interaction flows, accessibility considerations, and cross-browser compatibility approaches.

[No sources needed since this section summarizes without analyzing specific files]

## Project Structure
The frontend is a single-page app driven by an HTML entry point and a TypeScript module that manages state, DOM rendering, and IPC calls.

```mermaid
graph TB
A["index.html<br/>App shell and layout"] --> B["src/main.ts<br/>State, UI logic, IPC"]
A --> C["src/styles.css<br/>Glassmorphism, animations, responsive"]
B --> D["@tauri-apps/api/core.invoke<br/>Tauri commands"]
```

**Diagram sources**
- [index.html:1-10](file://index.html#L1-L10)
- [src/main.ts:1-10](file://src/main.ts#L1-L10)
- [src/styles.css:1-45](file://src/styles.css#L1-L45)

**Section sources**
- [README.md:35-40](file://README.md#L35-L40)
- [package.json:12-19](file://package.json#L12-L19)
- [index.html:1-10](file://index.html#L1-L10)

## Core Components
- State model: Hierarchical TreeNode structure with selection states (selected, indeterminate), expansion flags, and parent/child links.
- Data models: ScanResult, DiskUsage, DeleteResult, PlatformInfo, and related interfaces define the contract with the backend.
- UI components:
  - Disk usage ring and stats.
  - Results sections for Safe and Review categories.
  - Dynamic tree nodes and sub-items.
  - Confirmation dialog overlay.
  - Toast notification container.
- IPC layer: invoke() calls to scan_system, delete_items, get_platform_info, get_disk_usage, and scan_sub_items.

**Section sources**
- [src/main.ts:5-85](file://src/main.ts#L5-L85)
- [src/main.ts:87-97](file://src/main.ts#L87-L97)
- [index.html:25-57](file://index.html#L25-L57)
- [index.html:78-132](file://index.html#L78-L132)

## Architecture Overview
The frontend orchestrates scanning, selection, and deletion workflows while keeping the UI reactive to state changes.

```mermaid
sequenceDiagram
participant User as "User"
participant UI as "index.html"
participant App as "src/main.ts"
participant Tauri as "Tauri Backend"
User->>UI : Click "Scan System"
UI->>App : runScan()
App->>Tauri : invoke("scan_system")
Tauri-->>App : ScanResult
App->>App : updateDiskUsageUI()
App->>App : renderResults()
App-->>UI : Show results and totals
User->>UI : Select items / Expand tree
UI->>App : setNodeSelection(), toggleNodeExpand()
App->>App : updateAllCheckboxesDOM(), updateSelectionUI()
User->>UI : Click "Clean Selected"
UI->>App : runClean()
App->>App : showConfirmCleanupDialog()
App->>Tauri : invoke("delete_items", {paths})
Tauri-->>App : DeleteResult
App->>App : showToast()
App->>App : runScan()
```

**Diagram sources**
- [src/main.ts:837-896](file://src/main.ts#L837-L896)
- [src/main.ts:900-941](file://src/main.ts#L900-L941)
- [src/main.ts:945-1008](file://src/main.ts#L945-L1008)

## Detailed Component Analysis

### State Management Patterns for Hierarchical Tree Navigation
- Node model: TreeNode includes selection state (selected, indeterminate), expansion state (expanded), parent reference, and children array.
- Selection propagation:
  - Downward: When a node is selected/deselected, all descendants are updated accordingly.
  - Upward: Ancestors compute their state based on children’s selection status, supporting tri-state checkboxes.
- Lazy loading: Children are fetched on demand via Tauri when expanding a node; once loaded, they are cached in memory.

```mermaid
flowchart TD
Start(["setNodeSelection(node, checked)"]) --> SetSelf["Set node.selected and reset indeterminate"]
SetSelf --> Descendants["Recursively set descendants selected state"]
Descendants --> Ancestors["Walk up to ancestors"]
Ancestors --> CheckChildren{"All children selected?"}
CheckChildren --> |Yes| SetParentSelected["Set parent.selected = true, indeterminate = false"]
CheckChildren --> |No| CheckNone{"Any child selected?"}
CheckNone --> |No| SetParentUnselected["Set parent.selected = false, indeterminate = false"]
CheckNone --> |Yes| SetParentIndeterminate["Set parent.selected = false, indeterminate = true"]
SetParentSelected --> End(["Done"])
SetParentUnselected --> End
SetParentIndeterminate --> End
```

**Diagram sources**
- [src/main.ts:276-310](file://src/main.ts#L276-L310)

**Section sources**
- [src/main.ts:59-78](file://src/main.ts#L59-L78)
- [src/main.ts:272-310](file://src/main.ts#L272-L310)
- [src/main.ts:312-336](file://src/main.ts#L312-L336)
- [src/main.ts:338-414](file://src/main.ts#L338-L414)

### UI Rendering Components
- Dynamic tree structure generation:
  - Top-level category nodes rendered via createTopLevelItemElement.
  - Sub-item nodes rendered via createSubItemElement.
  - Expansion toggles lazy-load children using invoke("scan_sub_items").
- Confirmation dialogs:
  - showConfirmCleanupDialog builds a modal overlay with safe/review breakdowns and total reclaimable space.
- Toast notifications:
  - showToast creates transient messages with success/error/info types and auto-dismiss after delay.

```mermaid
classDiagram
class TreeNode {
+string id
+string name
+string path
+number size
+number file_count
+boolean is_dir
+boolean has_children
+string icon
+string description
+string safety_tip
+string safety_level
+boolean is_top_level
+TreeNode parent
+TreeNode[] children
+boolean childrenLoaded
+boolean selected
+boolean indeterminate
+boolean expanded
}
class ScanResult {
+DiskUsage disk_usage
+CleanupItem[] safe_items
+CleanupItem[] review_items
+number total_safe_size
+number total_review_size
}
class ConfirmDetails {
+number totalSize
+number pathsCount
+{name,size,path}[] safeItems
+{name,size,path}[] reviewItems
}
class DeleteResult {
+boolean success
+number deleted_count
+number freed_bytes
+string[] errors
}
ScanResult --> CleanupItem : "contains"
ConfirmDetails --> TreeNode : "derived from selection"
```

**Diagram sources**
- [src/main.ts:5-85](file://src/main.ts#L5-L85)
- [src/main.ts:276-310](file://src/main.ts#L276-L310)
- [src/main.ts:146-243](file://src/main.ts#L146-L243)

**Section sources**
- [src/main.ts:472-558](file://src/main.ts#L472-L558)
- [src/main.ts:560-624](file://src/main.ts#L560-L624)
- [src/main.ts:626-696](file://src/main.ts#L626-L696)
- [src/main.ts:700-833](file://src/main.ts#L700-L833)
- [src/main.ts:121-142](file://src/main.ts#L121-L142)
- [src/main.ts:146-243](file://src/main.ts#L146-L243)

### Tauri Command Integration Patterns
- Commands used:
  - scan_system: Returns ScanResult including disk usage and categorized items.
  - delete_items: Deletes selected paths and returns DeleteResult.
  - get_platform_info: Returns platform details to adapt UI labels.
  - get_disk_usage: Returns current disk usage metrics.
  - scan_sub_items: Returns nested items for a given directory path.
- Invocation pattern:
  - Use invoke<T>(commandName, payload?) to call backend functions.
  - Handle errors with try/catch and display toast notifications.

```mermaid
sequenceDiagram
participant App as "src/main.ts"
participant Tauri as "Tauri Backend"
App->>Tauri : invoke("get_platform_info")
Tauri-->>App : PlatformInfo
App->>Tauri : invoke("get_disk_usage")
Tauri-->>App : DiskUsage
App->>Tauri : invoke("scan_system")
Tauri-->>App : ScanResult
App->>Tauri : invoke("scan_sub_items", {path})
Tauri-->>App : SubCleanupItem[]
App->>Tauri : invoke("delete_items", {paths})
Tauri-->>App : DeleteResult
```

**Diagram sources**
- [src/main.ts:837-896](file://src/main.ts#L837-L896)
- [src/main.ts:900-941](file://src/main.ts#L900-L941)
- [src/main.ts:945-1008](file://src/main.ts#L945-L1008)
- [src/main.ts:499-549](file://src/main.ts#L499-L549)

**Section sources**
- [src/main.ts:1-10](file://src/main.ts#L1-L10)
- [src/main.ts:837-896](file://src/main.ts#L837-L896)
- [src/main.ts:900-941](file://src/main.ts#L900-L941)
- [src/main.ts:945-1008](file://src/main.ts#L945-L1008)
- [src/main.ts:499-549](file://src/main.ts#L499-L549)

### Glassmorphism CSS Styling and Responsive Design
- Glassmorphism approach:
  - Semi-transparent backgrounds with backdrop-filter blur for cards, action bar, toasts, and dialogs.
  - Subtle borders and shadows to enhance depth.
- Visual feedback:
  - Hover and active states for buttons and rows.
  - Tri-state custom checkboxes with distinct styles for checked and indeterminate states.
  - Animations for slide-up, fade-in, spin, pulse, and staggered item entrance.
- Responsive patterns:
  - Media queries adjust layout for small screens.
  - Flexible containers and text truncation ensure readability.

```mermaid
graph TB
A["Glass Cards<br/>.glass-card"] --> B["Backdrop Blur<br/>backdrop-filter"]
A --> C["Borders & Shadows<br/>border + box-shadow"]
D["Action Bar<br/>.action-bar"] --> B
E["Toast Notifications<br/>.toast"] --> B
F["Confirmation Dialog<br/>.dialog-overlay/.dialog"] --> B
G["Responsive Styles<br/>@media (max-width: 600px)"] --> H["Stacked Layouts<br/>Flex Direction Column"]
```

**Diagram sources**
- [src/styles.css:134-143](file://src/styles.css#L134-L143)
- [src/styles.css:753-825](file://src/styles.css#L753-L825)
- [src/styles.css:827-880](file://src/styles.css#L827-L880)
- [src/styles.css:987-1175](file://src/styles.css#L987-L1175)
- [src/styles.css:1176-1188](file://src/styles.css#L1176-L1188)

**Section sources**
- [src/styles.css:1-45](file://src/styles.css#L1-L45)
- [src/styles.css:134-143](file://src/styles.css#L134-L143)
- [src/styles.css:753-825](file://src/styles.css#L753-L825)
- [src/styles.css:827-880](file://src/styles.css#L827-L880)
- [src/styles.css:987-1175](file://src/styles.css#L987-L1175)
- [src/styles.css:1176-1188](file://src/styles.css#L1176-L1188)

### Event Handling, State Synchronization, and Component Composition
- Event handling:
  - Button clicks trigger runScan and runClean.
  - Row and chevron clicks handle expand and selection updates.
  - Select All buttons toggle all nodes within Safe or Review categories.
- State synchronization:
  - setNodeSelection updates both model and DOM via updateAllCheckboxesDOM.
  - updateSelectionUI recalculates counts, sizes, and visibility of the action bar.
- Component composition:
  - createTopLevelItemElement composes row elements with icons, badges, and metadata.
  - createSubItemElement composes nested items with optional expand chevrons.

```mermaid
sequenceDiagram
participant User as "User"
participant DOM as "DOM Elements"
participant App as "src/main.ts"
User->>DOM : Click row or checkbox
DOM->>App : click handler
App->>App : setNodeSelection(node, willSelect)
App->>App : updateAllCheckboxesDOM()
App->>App : updateSelectionUI()
App-->>DOM : Update classes and text content
User->>DOM : Click "Select All"
DOM->>App : select-all handler
App->>App : setNodeSelection(all nodes, targetState)
App->>App : updateAllCheckboxesDOM()
App->>App : updateSelectionUI()
```

**Diagram sources**
- [src/main.ts:600-608](file://src/main.ts#L600-L608)
- [src/main.ts:672-680](file://src/main.ts#L672-L680)
- [src/main.ts:982-1004](file://src/main.ts#L982-L1004)
- [src/main.ts:418-468](file://src/main.ts#L418-L468)

**Section sources**
- [src/main.ts:600-608](file://src/main.ts#L600-L608)
- [src/main.ts:672-680](file://src/main.ts#L672-L680)
- [src/main.ts:982-1004](file://src/main.ts#L982-L1004)
- [src/main.ts:418-468](file://src/main.ts#L418-L468)

### User Interaction Flows
- Initial load:
  - Auto-detect platform info and initial disk usage.
  - Auto-trigger scan on startup.
- Scanning flow:
  - Disable scan button, show scanning overlay with rotating status texts.
  - On completion, update disk usage UI, render results, and show toast.
- Deletion flow:
  - Build confirmation dialog with safe/review breakdowns.
  - If confirmed, call delete_items, show success/error toast, then re-scan.

```mermaid
flowchart TD
Start(["DOMContentLoaded"]) --> DetectPlatform["Invoke get_platform_info"]
DetectPlatform --> GetDiskUsage["Invoke get_disk_usage"]
GetDiskUsage --> AutoScan["runScan()"]
AutoScan --> ShowOverlay["Show scanning overlay"]
ShowOverlay --> InvokeScan["Invoke scan_system"]
InvokeScan --> RenderResults["renderResults()"]
RenderResults --> HideOverlay["Hide scanning overlay"]
HideOverlay --> Ready(["Ready for selection"])
Ready --> CleanFlow["Click 'Clean Selected'"]
CleanFlow --> ConfirmDialog["showConfirmCleanupDialog()"]
ConfirmDialog --> |Confirmed| InvokeDelete["Invoke delete_items"]
InvokeDelete --> ShowToast["showToast(success/error)"]
ShowToast --> ReScan["runScan()"]
ReScan --> Ready
```

**Diagram sources**
- [src/main.ts:945-1008](file://src/main.ts#L945-L1008)
- [src/main.ts:837-896](file://src/main.ts#L837-L896)
- [src/main.ts:900-941](file://src/main.ts#L900-L941)

**Section sources**
- [src/main.ts:945-1008](file://src/main.ts#L945-L1008)
- [src/main.ts:837-896](file://src/main.ts#L837-L896)
- [src/main.ts:900-941](file://src/main.ts#L900-L941)

### Accessibility Considerations
- Keyboard and focus:
  - Buttons and interactive rows should be reachable via keyboard; consider adding tabindex and focus styles.
- Semantic markup:
  - Use semantic tags (section, header, main) already present in index.html.
- ARIA attributes:
  - Add aria-checked for custom checkboxes to reflect tri-state behavior.
  - Add aria-expanded for expandable nodes.
- Color contrast:
  - Ensure sufficient contrast for text and badges against dark backgrounds.
- Screen readers:
  - Provide descriptive titles and alt text for icons where applicable.

[No sources needed since this section provides general guidance]

### Cross-Browser Compatibility Approaches
- Backdrop filter support:
  - Uses -webkit-backdrop-filter fallback alongside standard backdrop-filter.
- Scrollbar styling:
  - Custom scrollbar styles use WebKit pseudo-elements; consider non-WebKit fallbacks if targeting other browsers.
- Font smoothing:
  - Uses -webkit-font-smoothing and -moz-osx-font-smoothing for consistent typography across platforms.
- Viewport meta:
  - Includes viewport meta tag for mobile responsiveness.

**Section sources**
- [src/styles.css:38-45](file://src/styles.css#L38-L45)
- [src/styles.css:117-132](file://src/styles.css#L117-L132)
- [index.html:1-10](file://index.html#L1-L10)

## Dependency Analysis
Frontend dependencies include Tauri API modules for IPC and opener plugin. The build pipeline uses Vite and TypeScript.

```mermaid
graph LR
TS["src/main.ts"] --> TauriAPI["@tauri-apps/api/core.invoke"]
TS --> TauriOpener["@tauri-apps/plugin-opener"]
TS --> DOM["HTML DOM APIs"]
TS --> CSS["src/styles.css"]
HTML["index.html"] --> TS
HTML --> CSS
```

**Diagram sources**
- [package.json:12-19](file://package.json#L12-L19)
- [src/main.ts:1-10](file://src/main.ts#L1-L10)
- [index.html:1-10](file://index.html#L1-L10)

**Section sources**
- [package.json:12-19](file://package.json#L12-L19)
- [src/main.ts:1-10](file://src/main.ts#L1-L10)

## Performance Considerations
- Lazy loading of sub-items reduces initial payload and improves perceived performance.
- DOM updates are batched via targeted class toggles and selective innerHTML replacements.
- Avoid unnecessary re-renders by checking existing children before appending.
- Debounce or throttle repeated interactions if needed for large trees.

[No sources needed since this section provides general guidance]

## Troubleshooting Guide
- Scan failures:
  - Errors from scan_system are caught and displayed via toast; check backend logs and permissions.
- Deletion failures:
  - delete_items may return partial success; inspect DeleteResult.errors and freed_bytes.
- Platform detection issues:
  - get_platform_info errors fall back to default labels; verify Tauri configuration and capabilities.
- Tree expansion errors:
  - scan_sub_items failures show error messages in the tree; verify path validity and permissions.

**Section sources**
- [src/main.ts:876-896](file://src/main.ts#L876-L896)
- [src/main.ts:921-941](file://src/main.ts#L921-L941)
- [src/main.ts:945-970](file://src/main.ts#L945-L970)
- [src/main.ts:499-549](file://src/main.ts#L499-L549)

## Conclusion
FreeSpaceMacos frontend implements a robust, reactive architecture centered around a hierarchical TreeNode state model, dynamic DOM rendering, and Tauri IPC for backend operations. The glassmorphism design provides a modern macOS aesthetic with responsive layouts and clear visual feedback. By following the documented patterns for event handling, state synchronization, and component composition, developers can extend functionality while maintaining clarity and performance.

[No sources needed since this section summarizes without analyzing specific files]