import { invoke } from "@tauri-apps/api/core";

// ─── Interfaces ────────────────────────────────────────────────

interface SubCleanupItem {
  id: string;
  name: string;
  path: string;
  size: number;
  file_count: number;
  is_dir: boolean;
  has_children: boolean;
  children?: SubCleanupItem[];
}

interface CleanupItem {
  id: string;
  name: string;
  path: string;
  size: number;
  category: string;
  icon: string;
  safety_level: "Safe" | "NeedsReview";
  description: string;
  safety_tip: string;
  file_count: number;
  exists: boolean;
  children: SubCleanupItem[];
}

interface DiskUsage {
  total: number;
  used: number;
  free: number;
  percentage_used: number;
}

interface ScanResult {
  disk_usage: DiskUsage;
  safe_items: CleanupItem[];
  review_items: CleanupItem[];
  total_safe_size: number;
  total_review_size: number;
}

interface DeleteResult {
  success: boolean;
  deleted_count: number;
  freed_bytes: number;
  errors: string[];
}

interface PlatformInfo {
  os: string;
  os_name: string;
  disk_label: string;
}

interface LargeFile {
  name: string;
  path: string;
  size: number;
}

interface TreeNode {
  id: string;
  name: string;
  path: string;
  size: number;
  file_count: number;
  is_dir: boolean;
  has_children: boolean;
  icon?: string;
  description?: string;
  safety_tip?: string;
  safety_level: "Safe" | "NeedsReview";
  is_top_level: boolean;
  parent: TreeNode | null;
  children: TreeNode[];
  childrenLoaded: boolean;
  selected: boolean;
  indeterminate: boolean;
  expanded: boolean;
}

interface ConfirmDetails {
  totalSize: number;
  pathsCount: number;
  safeItems: { name: string; size: number; path: string }[];
  reviewItems: { name: string; size: number; path: string }[];
}

// ─── State ─────────────────────────────────────────────────────

let safeNodes: TreeNode[] = [];
let reviewNodes: TreeNode[] = [];
let isScanning = false;
let isDeleting = false;

// Platform state (auto-detected at startup; macOS is the default fallback)
let platformOs = "unknown";
let diskLabel = "Macintosh HD";

// ─── Tab / Size-Tree / Large-Files State ───────────────────────
type TabId = "cleanup" | "tree" | "files";
let treeLoaded = false;
let treeLoading = false;
let filesLoaded = false;
let filesLoading = false;

// ─── Format Utilities ──────────────────────────────────────────

function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
}

function getFileIcon(name: string): string {
  const lower = name.toLowerCase();
  if (lower.endsWith(".log") || lower.endsWith(".txt")) return "📋";
  if (lower.endsWith(".zip") || lower.endsWith(".tar") || lower.endsWith(".gz") || lower.endsWith(".tgz")) return "📦";
  if (lower.endsWith(".plist") || lower.endsWith(".json") || lower.endsWith(".xml") || lower.endsWith(".toml")) return "⚙️";
  if (lower.endsWith(".png") || lower.endsWith(".jpg") || lower.endsWith(".jpeg") || lower.endsWith(".webp") || lower.endsWith(".icns")) return "🖼️";
  if (lower.endsWith(".db") || lower.endsWith(".sqlite") || lower.endsWith(".sql")) return "🗄️";
  if (lower.endsWith(".app") || lower.endsWith(".framework") || lower.endsWith(".dylib")) return "📦";
  if (lower.endsWith(".exe") || lower.endsWith(".dll") || lower.endsWith(".msi")) return "📦";
  return "📄";
}

// ─── Toast System ──────────────────────────────────────────────

function showToast(message: string, type: "success" | "error" | "info" = "info") {
  const container = document.getElementById("toast-container");
  if (!container) return;

  const toast = document.createElement("div");
  toast.className = `toast ${type}`;

  const icon = type === "success" ? "✨" : type === "error" ? "❌" : "ℹ️";
  toast.innerHTML = `
    <span class="toast-icon">${icon}</span>
    <span class="toast-msg">${message}</span>
  `;

  container.appendChild(toast);

  setTimeout(() => {
    toast.classList.add("exit");
    setTimeout(() => toast.remove(), 300);
  }, 4000);
}

// ─── Confirmation Dialog ───────────────────────────────────────

function showConfirmCleanupDialog(details: ConfirmDetails): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "dialog-overlay";

    let safeSectionHtml = "";
    if (details.safeItems.length > 0) {
      const safeTotal = details.safeItems.reduce((acc, i) => acc + i.size, 0);
      safeSectionHtml = `
        <div class="dialog-group-title safe">
          <span class="dialog-group-badge safe">✅ Safe to Clean (${formatBytes(safeTotal)})</span>
          <span class="dialog-group-sub">🟢 An toàn 100%: Dữ liệu cache & tạm thời, hệ thống tự tái tạo khi cần, không mất dữ liệu.</span>
        </div>
        ${details.safeItems
          .map(
            (i) => `
          <div class="dialog-item-row">
            <span class="dialog-item-name" title="${i.path}">• ${i.name}</span>
            <span class="dialog-item-size">${formatBytes(i.size)}</span>
          </div>`
          )
          .join("")}
      `;
    }

    let reviewSectionHtml = "";
    if (details.reviewItems.length > 0) {
      const reviewTotal = details.reviewItems.reduce((acc, i) => acc + i.size, 0);
      reviewSectionHtml = `
        <div class="dialog-group-title review">
          <span class="dialog-group-badge review">⚠️ Needs Review (${formatBytes(reviewTotal)})</span>
          <span class="dialog-group-sub">🟡 Cần xem xét: Build artifacts, máy ảo Simulator, Downloads, Model AI... Tải/build lại khi dùng.</span>
        </div>
        ${details.reviewItems
          .map(
            (i) => `
          <div class="dialog-item-row">
            <span class="dialog-item-name" title="${i.path}">• ${i.name}</span>
            <span class="dialog-item-size">${formatBytes(i.size)}</span>
          </div>`
          )
          .join("")}
      `;
    }

    const warningHtml =
      details.reviewItems.length > 0
        ? `<div class="dialog-warning-note">
             <span class="dialog-warning-icon">⚠️</span>
             <div>Một số mục đã chọn thuộc diện <strong>Cần xem xét</strong> (build artifacts, máy ảo Simulator, Downloads, Model AI). Bạn có thể mở rộng danh sách ở màn hình chính để bỏ chọn các file quan trọng trước khi xóa.</div>
           </div>`
        : `<div class="dialog-safe-note">
             <span>🛡️</span>
             <div>Tất cả mục đã chọn đều là <strong>100% An toàn</strong>. Bạn có thể yên tâm dọn dẹp để giải phóng bộ nhớ ngay lập tức.</div>
           </div>`;

    overlay.innerHTML = `
      <div class="dialog">
        <div class="dialog-header">
          <h3>🧹 Confirm Space Cleanup</h3>
        </div>

        <div class="dialog-highlight-card">
          <span class="dialog-highlight-label">Reclaimable Space</span>
          <span class="dialog-highlight-value">${formatBytes(details.totalSize)}</span>
        </div>

        <div class="dialog-list-container">
          ${safeSectionHtml}
          ${reviewSectionHtml}
        </div>

        ${warningHtml}

        <p class="dialog-footer-text">
          ${details.pathsCount} ${details.pathsCount === 1 ? "item/folder" : "items/folders"} will be permanently removed to reclaim disk space.
        </p>

        <div class="dialog-actions">
          <button id="dialog-cancel" class="dialog-btn cancel">Cancel</button>
          <button id="dialog-confirm" class="dialog-btn confirm">Clean ${formatBytes(details.totalSize)}</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    overlay.querySelector("#dialog-cancel")?.addEventListener("click", () => {
      overlay.remove();
      resolve(false);
    });

    overlay.querySelector("#dialog-confirm")?.addEventListener("click", () => {
      overlay.remove();
      resolve(true);
    });
  });
}

// ─── Disk Usage UI ─────────────────────────────────────────────

function updateDiskUsageUI(usage: DiskUsage) {
  const labelEl = document.getElementById("disk-label");
  const percentEl = document.getElementById("disk-percent");
  const statUsedEl = document.getElementById("stat-used");
  const statFreeEl = document.getElementById("stat-free");
  const statTotalEl = document.getElementById("stat-total");
  const ringProgress = document.getElementById("ring-progress") as SVGCircleElement | null;

  const pct = Math.min(100, Math.max(0, usage.percentage_used));

  if (labelEl) labelEl.textContent = `${diskLabel} · ${formatBytes(usage.free)} Available`;
  if (percentEl) percentEl.textContent = `${pct.toFixed(0)}%`;
  if (statUsedEl) statUsedEl.textContent = formatBytes(usage.used);
  if (statFreeEl) statFreeEl.textContent = formatBytes(usage.free);
  if (statTotalEl) statTotalEl.textContent = formatBytes(usage.total);

  if (ringProgress) {
    const circumference = 2 * Math.PI * 52;
    const offset = circumference - (pct / 100) * circumference;
    ringProgress.style.strokeDashoffset = `${offset}`;
  }
}

// ─── Hierarchical Selection Model ──────────────────────────────

function getAllTopLevelNodes(): TreeNode[] {
  return [...safeNodes, ...reviewNodes];
}

function setNodeSelection(node: TreeNode, checked: boolean) {
  node.selected = checked;
  node.indeterminate = false;

  // Propagate downwards to all descendants
  function setDescendants(n: TreeNode, val: boolean) {
    n.selected = val;
    n.indeterminate = false;
    for (const child of n.children) {
      setDescendants(child, val);
    }
  }
  for (const child of node.children) {
    setDescendants(child, checked);
  }

  // Propagate upwards to ancestors
  let curr = node.parent;
  while (curr) {
    const allChecked = curr.children.length > 0 && curr.children.every((c) => c.selected && !c.indeterminate);
    const noneChecked = curr.children.every((c) => !c.selected && !c.indeterminate);

    if (allChecked) {
      curr.selected = true;
      curr.indeterminate = false;
    } else if (noneChecked) {
      curr.selected = false;
      curr.indeterminate = false;
    } else {
      curr.selected = false;
      curr.indeterminate = true;
    }
    curr = curr.parent;
  }
}

function updateCheckboxDOM(node: TreeNode) {
  const el = document.querySelector(`[data-node-id="${CSS.escape(node.id)}"]`);
  if (!el) return;

  const checkbox = el.querySelector(".item-checkbox");
  if (checkbox) {
    checkbox.classList.toggle("checked", node.selected);
    checkbox.classList.toggle("indeterminate", node.indeterminate);
  }

  el.classList.toggle("selected", node.selected);
  el.classList.toggle("partial", node.indeterminate);
}

function updateAllCheckboxesDOM() {
  function update(node: TreeNode) {
    updateCheckboxDOM(node);
    for (const child of node.children) {
      update(child);
    }
  }
  for (const n of getAllTopLevelNodes()) {
    update(n);
  }
}

function getSelectedPathsToDelete(): string[] {
  const paths: string[] = [];

  function collect(node: TreeNode) {
    // If fully checked without indeterminate, delete this root or directory directly
    if (node.selected && !node.indeterminate) {
      paths.push(node.path);
      return;
    }

    // If indeterminate, recurse into children to collect only checked items
    if (node.indeterminate) {
      for (const child of node.children) {
        collect(child);
      }
    }
  }

  for (const root of getAllTopLevelNodes()) {
    collect(root);
  }

  return paths;
}

function getSelectedSize(): number {
  let total = 0;

  function collectSize(node: TreeNode) {
    if (node.selected && !node.indeterminate) {
      total += node.size;
      return;
    }
    if (node.indeterminate) {
      for (const child of node.children) {
        collectSize(child);
      }
    }
  }

  for (const root of getAllTopLevelNodes()) {
    collectSize(root);
  }

  return total;
}

function getSelectedBreakdown(): {
  safeItems: { name: string; size: number; path: string }[];
  reviewItems: { name: string; size: number; path: string }[];
} {
  const safeItems: { name: string; size: number; path: string }[] = [];
  const reviewItems: { name: string; size: number; path: string }[] = [];

  function collectItems(node: TreeNode) {
    if (node.selected && !node.indeterminate) {
      const entry = { name: node.name, size: node.size, path: node.path };
      if (node.safety_level === "Safe") {
        safeItems.push(entry);
      } else {
        reviewItems.push(entry);
      }
      return;
    }
    if (node.indeterminate) {
      for (const child of node.children) {
        collectItems(child);
      }
    }
  }

  for (const root of getAllTopLevelNodes()) {
    collectItems(root);
  }

  return { safeItems, reviewItems };
}

// ─── Action Bar & Selection UI ─────────────────────────────────

function updateSelectionUI() {
  const countEl = document.getElementById("selected-count");
  const sizeEl = document.getElementById("selected-size");
  const cleanBtn = document.getElementById("btn-clean") as HTMLButtonElement | null;
  const cleanBtnText = document.getElementById("btn-clean-text");
  const actionBar = document.getElementById("action-bar");

  const paths = getSelectedPathsToDelete();
  const totalSelectedSize = getSelectedSize();

  if (countEl) {
    countEl.textContent = `${paths.length} ${paths.length === 1 ? "item" : "items"} selected`;
  }
  if (sizeEl) {
    sizeEl.textContent = formatBytes(totalSelectedSize);
  }

  if (cleanBtn) {
    cleanBtn.disabled = paths.length === 0 || isDeleting;
  }
  if (cleanBtnText) {
    cleanBtnText.textContent = isDeleting
      ? "Cleaning..."
      : `Clean Selected (${formatBytes(totalSelectedSize)})`;
  }

  if (actionBar) {
    if (paths.length > 0) {
      actionBar.classList.remove("hidden");
    } else {
      actionBar.classList.add("hidden");
    }
  }

  updateSelectAllButtons();
}

function updateSelectAllButtons() {
  const btnSafe = document.getElementById("btn-select-all-safe");
  const btnReview = document.getElementById("btn-select-all-review");

  if (btnSafe && safeNodes.length > 0) {
    const allSafeSelected = safeNodes.every((n) => n.selected && !n.indeterminate);
    btnSafe.textContent = allSafeSelected ? "Deselect All" : "Select All";
  }

  if (btnReview && reviewNodes.length > 0) {
    const allReviewSelected = reviewNodes.every((n) => n.selected && !n.indeterminate);
    btnReview.textContent = allReviewSelected ? "Deselect All" : "Select All";
  }
}

// ─── Tree Node Rendering & Expansion ───────────────────────────

async function toggleNodeExpand(node: TreeNode, nodeWrapperEl: HTMLElement) {
  node.expanded = !node.expanded;

  const chevron = nodeWrapperEl.querySelector(`.tree-chevron[data-node-id="${CSS.escape(node.id)}"]`);
  if (chevron) {
    chevron.classList.toggle("expanded", node.expanded);
  }

  let childrenContainer = nodeWrapperEl.querySelector(`.tree-children[data-parent-id="${CSS.escape(node.id)}"]`) as HTMLElement | null;

  if (!node.expanded) {
    if (childrenContainer) {
      childrenContainer.classList.add("collapsed");
    }
    return;
  }

  // Node is expanding
  if (!childrenContainer) {
    childrenContainer = document.createElement("div");
    childrenContainer.className = "tree-children";
    childrenContainer.setAttribute("data-parent-id", node.id);
    nodeWrapperEl.appendChild(childrenContainer);
  }

  childrenContainer.classList.remove("collapsed");

  // If children not loaded yet, fetch them from backend
  if (!node.childrenLoaded && (node.has_children || node.is_dir)) {
    childrenContainer.innerHTML = `
      <div class="tree-loading">
        <div class="tree-spinner"></div>
        <span>Loading contents...</span>
      </div>
    `;

    try {
      const subItems = await invoke<SubCleanupItem[]>("scan_sub_items", { path: node.path });
      node.childrenLoaded = true;

      node.children = subItems.map((item) => ({
        id: item.id,
        name: item.name,
        path: item.path,
        size: item.size,
        file_count: item.file_count,
        is_dir: item.is_dir,
        has_children: item.has_children,
        safety_level: node.safety_level,
        is_top_level: false,
        parent: node,
        children: [],
        childrenLoaded: false,
        selected: node.selected,
        indeterminate: false,
        expanded: false,
      }));

      childrenContainer.innerHTML = "";
      if (node.children.length === 0) {
        childrenContainer.innerHTML = `
          <div class="tree-empty">
            <span>Empty or no cache files</span>
          </div>
        `;
      } else {
        for (const childNode of node.children) {
          childrenContainer.appendChild(createSubItemElement(childNode));
        }
      }
    } catch (err) {
      console.error("Failed to load sub-items:", err);
      childrenContainer.innerHTML = `
        <div class="tree-empty">
          <span>Failed to load: ${err}</span>
        </div>
      `;
    }
  } else {
    // If already loaded, make sure children are rendered
    if (childrenContainer.children.length === 0 && node.children.length > 0) {
      for (const childNode of node.children) {
        childrenContainer.appendChild(createSubItemElement(childNode));
      }
    }
  }
}

function createSubItemElement(node: TreeNode): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "tree-sub-node-wrapper";

  const row = document.createElement("div");
  row.className = "tree-sub-item";
  row.setAttribute("data-node-id", node.id);
  if (node.selected) row.classList.add("selected");
  if (node.indeterminate) row.classList.add("partial");

  const canExpand = node.is_dir && (node.has_children || node.children.length > 0);
  const chevronHtml = canExpand
    ? `<button class="tree-chevron ${node.expanded ? "expanded" : ""}" data-node-id="${node.id}" title="Expand folder">
         <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
       </button>`
    : `<span class="tree-chevron hidden-placeholder"></span>`;

  const icon = node.is_dir ? "📁" : getFileIcon(node.name);

  row.innerHTML = `
    ${chevronHtml}
    <div class="item-checkbox ${node.selected ? "checked" : ""} ${node.indeterminate ? "indeterminate" : ""}" data-node-id="${node.id}"></div>
    <div class="sub-item-icon">${icon}</div>
    <div class="sub-item-info">
      <div class="sub-item-name" title="${node.name}">${node.name}</div>
      <div class="sub-item-path" title="${node.path}">${node.path}</div>
    </div>
    <div class="sub-item-meta">
      <div class="sub-item-size">${formatBytes(node.size)}</div>
      <div class="sub-item-files">${node.is_dir && node.file_count > 0 ? `${node.file_count.toLocaleString()} files` : ""}</div>
    </div>
  `;

  // Chevron click handler
  const chevronBtn = row.querySelector(".tree-chevron:not(.hidden-placeholder)");
  chevronBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleNodeExpand(node, wrapper);
  });

  // Checkbox & Row click handler
  row.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest(".tree-chevron")) return;

    const willSelect = !node.selected;
    setNodeSelection(node, willSelect);
    updateAllCheckboxesDOM();
    updateSelectionUI();
  });

  wrapper.appendChild(row);

  // If node was expanded already, render children container
  if (node.expanded && node.children.length > 0) {
    const childrenContainer = document.createElement("div");
    childrenContainer.className = "tree-children";
    childrenContainer.setAttribute("data-parent-id", node.id);
    for (const child of node.children) {
      childrenContainer.appendChild(createSubItemElement(child));
    }
    wrapper.appendChild(childrenContainer);
  }

  return wrapper;
}

function createTopLevelItemElement(node: TreeNode): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "tree-node-wrapper";

  const row = document.createElement("div");
  row.className = "cleanup-item";
  row.setAttribute("data-node-id", node.id);
  if (node.selected) row.classList.add("selected");
  if (node.indeterminate) row.classList.add("partial");

  const hasChildren = node.has_children || node.children.length > 0;
  const chevronHtml = hasChildren
    ? `<button class="tree-chevron ${node.expanded ? "expanded" : ""}" data-node-id="${node.id}" title="Expand category">
         <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
       </button>`
    : `<span class="tree-chevron hidden-placeholder"></span>`;

  const isSafe = node.safety_level === "Safe";
  const badgeClass = isSafe ? "safe" : "review";
  const badgeText = isSafe ? "🟢 100% An toàn" : "🟡 Cần xem xét";

  row.innerHTML = `
    ${chevronHtml}
    <div class="item-checkbox ${node.selected ? "checked" : ""} ${node.indeterminate ? "indeterminate" : ""}" data-node-id="${node.id}"></div>
    <div class="item-icon">${node.icon || "📦"}</div>
    <div class="item-info">
      <div class="item-name-row">
        <span class="item-name">${node.name}</span>
        <span class="item-safety-badge ${badgeClass}">${badgeText}</span>
      </div>
      <div class="item-desc" title="${node.description} (${node.path})">${node.description}</div>
      ${node.safety_tip ? `<div class="item-safety-tip">${node.safety_tip}</div>` : ""}
    </div>
    <div class="item-meta">
      <div class="item-size">${formatBytes(node.size)}</div>
      <div class="item-files">${node.file_count.toLocaleString()} files</div>
    </div>
  `;

  // Chevron click handler
  const chevronBtn = row.querySelector(".tree-chevron:not(.hidden-placeholder)");
  chevronBtn?.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleNodeExpand(node, wrapper);
  });

  // Checkbox / row click handler
  row.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest(".tree-chevron")) return;

    const willSelect = !node.selected;
    setNodeSelection(node, willSelect);
    updateAllCheckboxesDOM();
    updateSelectionUI();
  });

  wrapper.appendChild(row);

  // If node was expanded already, render children container
  if (node.expanded && node.children.length > 0) {
    const childrenContainer = document.createElement("div");
    childrenContainer.className = "tree-children";
    childrenContainer.setAttribute("data-parent-id", node.id);
    for (const child of node.children) {
      childrenContainer.appendChild(createSubItemElement(child));
    }
    wrapper.appendChild(childrenContainer);
  }

  return wrapper;
}

// ─── Render Results List ───────────────────────────────────────

function renderResults(result: ScanResult) {
  const resultsContainer = document.getElementById("results");
  const safeList = document.getElementById("safe-items");
  const reviewList = document.getElementById("review-items");
  const safeSizeEl = document.getElementById("safe-total-size");
  const reviewSizeEl = document.getElementById("review-total-size");

  if (!resultsContainer || !safeList || !reviewList) return;

  safeList.innerHTML = "";
  reviewList.innerHTML = "";

  if (safeSizeEl) safeSizeEl.textContent = formatBytes(result.total_safe_size);
  if (reviewSizeEl) reviewSizeEl.textContent = formatBytes(result.total_review_size);

  // Build Safe TreeNodes (default: selected = true)
  safeNodes = result.safe_items.map((item) => {
    const parentNode: TreeNode = {
      id: item.id,
      name: item.name,
      path: item.path,
      size: item.size,
      file_count: item.file_count,
      icon: item.icon,
      description: item.description,
      safety_tip: item.safety_tip,
      safety_level: item.safety_level,
      is_top_level: true,
      is_dir: true,
      has_children: item.children.length > 0,
      parent: null,
      children: [],
      childrenLoaded: true,
      selected: true,
      indeterminate: false,
      expanded: false,
    };

    parentNode.children = item.children.map((child) => ({
      id: child.id,
      name: child.name,
      path: child.path,
      size: child.size,
      file_count: child.file_count,
      is_dir: child.is_dir,
      has_children: child.has_children,
      safety_level: item.safety_level,
      is_top_level: false,
      parent: parentNode,
      children: [],
      childrenLoaded: false,
      selected: true,
      indeterminate: false,
      expanded: false,
    }));

    return parentNode;
  });

  // Build Review TreeNodes (default: selected = false)
  reviewNodes = result.review_items.map((item) => {
    const parentNode: TreeNode = {
      id: item.id,
      name: item.name,
      path: item.path,
      size: item.size,
      file_count: item.file_count,
      icon: item.icon,
      description: item.description,
      safety_tip: item.safety_tip,
      safety_level: item.safety_level,
      is_top_level: true,
      is_dir: true,
      has_children: item.children.length > 0,
      parent: null,
      children: [],
      childrenLoaded: true,
      selected: false,
      indeterminate: false,
      expanded: false,
    };

    parentNode.children = item.children.map((child) => ({
      id: child.id,
      name: child.name,
      path: child.path,
      size: child.size,
      file_count: child.file_count,
      is_dir: child.is_dir,
      has_children: child.has_children,
      safety_level: item.safety_level,
      is_top_level: false,
      parent: parentNode,
      children: [],
      childrenLoaded: false,
      selected: false,
      indeterminate: false,
      expanded: false,
    }));

    return parentNode;
  });

  // Populate Safe items DOM
  if (safeNodes.length === 0) {
    safeList.innerHTML = `
      <div class="empty-state">
        <span class="empty-icon">🎉</span>
        <span class="empty-text">No system caches or logs need cleaning</span>
      </div>
    `;
  } else {
    for (const node of safeNodes) {
      safeList.appendChild(createTopLevelItemElement(node));
    }
  }

  // Populate Review items DOM
  if (reviewNodes.length === 0) {
    reviewList.innerHTML = `
      <div class="empty-state">
        <span class="empty-icon">✨</span>
        <span class="empty-text">No developer or app caches found</span>
      </div>
    `;
  } else {
    for (const node of reviewNodes) {
      reviewList.appendChild(createTopLevelItemElement(node));
    }
  }

  resultsContainer.classList.remove("hidden");
  updateSelectionUI();
}

// ─── Scan Action ───────────────────────────────────────────────

async function runScan() {
  if (isScanning || isDeleting) return;

  isScanning = true;
  const scanBtn = document.getElementById("btn-scan") as HTMLButtonElement | null;
  const scanOverlay = document.getElementById("scanning-overlay");
  const resultsContainer = document.getElementById("results");
  const scanningStatus = document.getElementById("scanning-status");

  if (scanBtn) {
    scanBtn.disabled = true;
    scanBtn.classList.add("scanning");
  }

  if (resultsContainer) resultsContainer.classList.add("hidden");
  if (scanOverlay) scanOverlay.classList.remove("hidden");

  const statusTexts =
    platformOs === "windows"
      ? [
          "Analyzing User & Windows Temp files...",
          "Scanning Browser & Developer caches...",
          "Checking Error reports & Update cache...",
          "Calculating directory sizes...",
        ]
      : [
          "Analyzing User & System Caches...",
          "Scanning Developer caches & archives...",
          "Checking Log files & Crash reports...",
          "Calculating directory sizes...",
        ];
  let statusIndex = 0;
  const interval = setInterval(() => {
    if (scanningStatus) {
      statusIndex = (statusIndex + 1) % statusTexts.length;
      scanningStatus.textContent = statusTexts[statusIndex];
    }
  }, 600);

  try {
    const result = await invoke<ScanResult>("scan_system");
    updateDiskUsageUI(result.disk_usage);
    renderResults(result);
    showToast(
      `Scan complete: Found ${formatBytes(result.total_safe_size + result.total_review_size)} reclaimable space`,
      "info"
    );
  } catch (err) {
    console.error("Scan error:", err);
    showToast(`Scan failed: ${err}`, "error");
  } finally {
    clearInterval(interval);
    isScanning = false;
    if (scanOverlay) scanOverlay.classList.add("hidden");
    if (scanBtn) {
      scanBtn.disabled = false;
      scanBtn.classList.remove("scanning");
    }
  }
}

// ─── Clean Action ──────────────────────────────────────────────

async function runClean() {
  if (isDeleting) return;

  const pathsToDelete = getSelectedPathsToDelete();
  if (pathsToDelete.length === 0) return;

  const totalSelectedSize = getSelectedSize();
  const { safeItems, reviewItems } = getSelectedBreakdown();

  const confirmed = await showConfirmCleanupDialog({
    totalSize: totalSelectedSize,
    pathsCount: pathsToDelete.length,
    safeItems,
    reviewItems,
  });

  if (!confirmed) return;

  isDeleting = true;
  updateSelectionUI();

  try {
    const result = await invoke<DeleteResult>("delete_items", { paths: pathsToDelete });
    if (result.success || result.freed_bytes > 0) {
      showToast(
        `Cleanup successful! Freed ${formatBytes(result.freed_bytes)} across ${result.deleted_count} items`,
        "success"
      );
    } else if (result.errors.length > 0) {
      showToast(`Finished with issues: ${result.errors[0]}`, "error");
    }

    // Refresh disk usage & re-scan
    await runScan();
  } catch (err) {
    console.error("Clean error:", err);
    showToast(`Clean failed: ${err}`, "error");
  } finally {
    isDeleting = false;
    updateSelectionUI();
  }
}

// ─── Tab Switching ─────────────────────────────────────────────

function switchTab(tab: TabId) {
  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.classList.toggle("active", (btn as HTMLElement).dataset.tab === tab);
  });
  const panels: Record<TabId, string> = {
    cleanup: "tab-cleanup",
    tree: "tab-tree",
    files: "tab-files",
  };
  (Object.keys(panels) as TabId[]).forEach((key) => {
    const el = document.getElementById(panels[key]);
    if (el) el.classList.toggle("hidden", key !== tab);
  });

  // Lazy-init heavy tabs on first visit
  if (tab === "tree" && !treeLoaded && !treeLoading) {
    initSizeTree();
  }
  if (tab === "files" && !filesLoaded && !filesLoading) {
    runLargeFileScan();
  }
}

// ─── Size-Sorted Directory Tree ────────────────────────────────

async function initSizeTree() {
  const container = document.getElementById("tree-container");
  const status = document.getElementById("tree-status");
  const btn = document.getElementById("btn-tree-refresh") as HTMLButtonElement | null;
  if (!container) return;

  treeLoading = true;
  if (btn) btn.disabled = true;
  if (status) status.textContent = "Đang quét dung lượng thư mục Home... (đây có thể mất chút thời gian)";
  container.innerHTML = `
    <div class="tree-loading" style="padding:20px">
      <div class="tree-spinner"></div>
      <span>Đang tính kích thước thư mục...</span>
    </div>
  `;

  try {
    const root = await invoke<SubCleanupItem>("get_tree_root");
    container.innerHTML = "";
    const maxRef = { size: root.size > 0 ? root.size : 1 };
    container.appendChild(buildSizeTreeNode(root, maxRef, true));
    treeLoaded = true;
    if (status) {
      status.textContent = `Home · ${formatBytes(root.size)} · ${root.file_count.toLocaleString()} tệp. Nhấn mũi tên để mở rộng (tải dần).`;
    }
  } catch (err) {
    console.error("Tree load error:", err);
    container.innerHTML = `<div class="tree-empty" style="padding:20px">Lỗi tải cây thư mục: ${err}</div>`;
    if (status) status.textContent = "Không thể tải cây thư mục.";
  } finally {
    treeLoading = false;
    if (btn) btn.disabled = false;
  }
}

function buildSizeTreeNode(item: SubCleanupItem, maxRef: { size: number }, isRoot = false): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "tree-sub-node-wrapper";

  const row = document.createElement("div");
  row.className = "tree-sub-item";

  const canExpand = item.is_dir && item.has_children;
  const chevronHtml = canExpand
    ? `<button class="tree-chevron" title="Mở rộng">
         <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
       </button>`
    : `<span class="tree-chevron hidden-placeholder"></span>`;

  const icon = item.is_dir ? "📁" : getFileIcon(item.name);
  const pct = maxRef.size > 0 ? Math.max(2, Math.min(100, Math.round((item.size / maxRef.size) * 100))) : 0;
  const barClass = pct >= 60 ? "st-bar top" : "st-bar";

  row.innerHTML = `
    ${chevronHtml}
    <div class="sub-item-icon">${icon}</div>
    <div class="sub-item-info">
      <div class="sub-item-name" title="${item.name}">${item.name}</div>
      <div class="sub-item-path" title="${item.path}">${item.path}</div>
    </div>
    <div class="st-bar-wrap"><div class="${barClass}" style="width:${pct}%"></div></div>
    <div class="sub-item-meta">
      <div class="sub-item-size">${formatBytes(item.size)}</div>
      <div class="sub-item-files">${item.is_dir && item.file_count > 0 ? `${item.file_count.toLocaleString()} files` : ""}</div>
    </div>
  `;

  wrapper.appendChild(row);

  if (canExpand) {
    const childrenContainer = document.createElement("div");
    childrenContainer.className = "tree-children collapsed";
    wrapper.appendChild(childrenContainer);

    let loaded = false;
    let expanded = false;
    const toggle = async () => {
      expanded = !expanded;
      const chev = row.querySelector(".tree-chevron");
      if (chev) chev.classList.toggle("expanded", expanded);
      childrenContainer.classList.toggle("collapsed", !expanded);

      if (expanded && !loaded) {
        loaded = true;
        childrenContainer.innerHTML = `
          <div class="tree-loading">
            <div class="tree-spinner"></div>
            <span>Đang tải...</span>
          </div>`;
        try {
          const children = await invoke<SubCleanupItem[]>("scan_sub_items", { path: item.path });
          childrenContainer.innerHTML = "";
          if (children.length === 0) {
            loaded = false;
            childrenContainer.innerHTML = `<div class="tree-empty"><span>Trống</span></div>`;
          } else {
            // Size bar relative to the largest sibling in this folder
            const siblingMax = { size: children[0]?.size ?? 1 };
            for (const child of children) {
              childrenContainer.appendChild(buildSizeTreeNode(child, siblingMax));
            }
          }
        } catch (err) {
          loaded = false;
          childrenContainer.innerHTML = `<div class="tree-empty"><span>Lỗi tải: ${err}</span></div>`;
        }
      }
    };

    row.addEventListener("click", (e) => {
      if ((e.target as HTMLElement).closest(".tree-chevron")) return;
      toggle();
    });
    row.querySelector(".tree-chevron")?.addEventListener("click", (e) => {
      e.stopPropagation();
      toggle();
    });

    if (isRoot) {
      // Auto-expand the root level for immediate overview
      toggle();
    }
  }

  return wrapper;
}

// ─── Large Files ───────────────────────────────────────────────

async function runLargeFileScan() {
  const container = document.getElementById("files-container");
  const status = document.getElementById("files-status");
  const btn = document.getElementById("btn-scan-files") as HTMLButtonElement | null;
  const sizeSel = document.getElementById("file-min-size") as HTMLSelectElement | null;
  if (!container) return;

  const minSizeMb = sizeSel ? parseInt(sizeSel.value, 10) || 100 : 100;

  filesLoading = true;
  if (btn) btn.disabled = true;
  if (status) status.textContent = `Đang quét các tệp ≥ ${minSizeMb} MB trong Home... (có thể mất chút thời gian)`;
  container.innerHTML = `
    <div class="tree-loading" style="padding:20px">
      <div class="tree-spinner"></div>
      <span>Đang tìm các tệp lớn...</span>
    </div>
  `;

  try {
    const files = await invoke<LargeFile[]>("scan_large_files", { minSizeMb, limit: 200 });
    container.innerHTML = "";
    if (files.length === 0) {
      container.innerHTML = `<div class="files-empty">Không tìm thấy tệp nào ≥ ${minSizeMb} MB.</div>`;
    } else {
      const revealLabel = platformOs === "windows" ? "Explorer" : "Finder";
      files.forEach((f, idx) => {
        container.appendChild(buildLargeFileRow(f, idx + 1, revealLabel));
      });
    }
    filesLoaded = true;
    if (status) status.textContent = `Tìm thấy ${files.length} tệp ≥ ${minSizeMb} MB (sắp xếp lớn → nhỏ).`;
  } catch (err) {
    console.error("Large files scan error:", err);
    container.innerHTML = `<div class="files-empty">Lỗi quét tệp lớn: ${err}</div>`;
    if (status) status.textContent = "Không thể quét tệp lớn.";
  } finally {
    filesLoading = false;
    if (btn) btn.disabled = false;
  }
}

function buildLargeFileRow(f: LargeFile, rank: number, revealLabel: string): HTMLElement {
  const row = document.createElement("div");
  row.className = "lf-row";
  const dir = f.path.replace(/[\\/][^\\/]*[\\/]?$/, "") || f.path;
  row.innerHTML = `
    <span class="lf-rank">${rank}</span>
    <span class="lf-icon">${getFileIcon(f.name)}</span>
    <div class="lf-info">
      <div class="lf-name" title="${f.name}">${f.name}</div>
      <div class="lf-path" title="${f.path}">${dir}</div>
    </div>
    <span class="lf-size">${formatBytes(f.size)}</span>
    <button class="reveal-btn" title="Hiển thị trong ${revealLabel}">📂 Reveal</button>
  `;
  row.querySelector(".reveal-btn")?.addEventListener("click", async (e) => {
    e.stopPropagation();
    const btnEl = e.currentTarget as HTMLElement;
    btnEl.textContent = "...";
    try {
      await invoke("reveal_path", { path: f.path });
    } catch (err) {
      console.error("Reveal error:", err);
      showToast(`Không hiển thị được: ${err}`, "error");
    } finally {
      btnEl.innerHTML = `📂 Reveal`;
    }
  });
  return row;
}

// ─── Initial Load & Event Listeners ────────────────────────────

window.addEventListener("DOMContentLoaded", async () => {
  // Auto-detect platform (macOS / Windows) and adapt labels & copy
  try {
    const info = await invoke<PlatformInfo>("get_platform_info");
    platformOs = info.os;
    diskLabel = info.disk_label;

    const badgeEl = document.getElementById("app-badge");
    if (badgeEl) {
      badgeEl.textContent = info.os === "windows" ? "Windows Cleaner" : "macOS Cleaner";
    }

    const scanningTextEl = document.getElementById("scanning-text");
    if (scanningTextEl && info.os === "windows") {
      scanningTextEl.textContent = "Đang phân tích sâu ổ đĩa hệ thống Windows...";
    }
  } catch (err) {
    console.error("Platform info error:", err);
  }

  try {
    const initialDisk = await invoke<DiskUsage>("get_disk_usage");
    updateDiskUsageUI(initialDisk);
  } catch (err) {
    console.error("Initial disk usage error:", err);
  }

  // Scan Button
  document.getElementById("btn-scan")?.addEventListener("click", () => {
    runScan();
  });

  // Clean Button
  document.getElementById("btn-clean")?.addEventListener("click", () => {
    runClean();
  });

  // Select All Safe
  document.getElementById("btn-select-all-safe")?.addEventListener("click", () => {
    if (safeNodes.length === 0) return;
    const allSafeSelected = safeNodes.every((n) => n.selected && !n.indeterminate);
    const targetState = !allSafeSelected;
    for (const node of safeNodes) {
      setNodeSelection(node, targetState);
    }
    updateAllCheckboxesDOM();
    updateSelectionUI();
  });

  // Select All Review
  document.getElementById("btn-select-all-review")?.addEventListener("click", () => {
    if (reviewNodes.length === 0) return;
    const allReviewSelected = reviewNodes.every((n) => n.selected && !n.indeterminate);
    const targetState = !allReviewSelected;
    for (const node of reviewNodes) {
      setNodeSelection(node, targetState);
    }
    updateAllCheckboxesDOM();
    updateSelectionUI();
  });

  // Auto trigger initial scan on startup
  runScan();

  // Tab switching
  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const t = (btn as HTMLElement).dataset.tab as TabId | undefined;
      if (t) switchTab(t);
    });
  });

  // Size-tree refresh
  document.getElementById("btn-tree-refresh")?.addEventListener("click", () => {
    initSizeTree();
  });

  // Large-files scan
  document.getElementById("btn-scan-files")?.addEventListener("click", () => {
    runLargeFileScan();
  });
});
