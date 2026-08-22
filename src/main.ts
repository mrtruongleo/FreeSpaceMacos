import { invoke } from "@tauri-apps/api/core";

// ─── Interfaces ────────────────────────────────────────────────

interface CleanupItem {
  id: string;
  name: string;
  path: string;
  size: number;
  category: string;
  icon: string;
  safety_level: "Safe" | "NeedsReview";
  description: string;
  file_count: number;
  exists: boolean;
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

// ─── State ─────────────────────────────────────────────────────

let currentScanResult: ScanResult | null = null;
const selectedItemIds = new Set<string>();
let isScanning = false;
let isDeleting = false;

// ─── Format Utilities ──────────────────────────────────────────

function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + " " + sizes[i];
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

function showConfirmDialog(title: string, message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "dialog-overlay";
    overlay.innerHTML = `
      <div class="dialog">
        <h3>${title}</h3>
        <p>${message}</p>
        <div class="dialog-actions">
          <button id="dialog-cancel" class="dialog-btn cancel">Cancel</button>
          <button id="dialog-confirm" class="dialog-btn confirm">Proceed Clean</button>
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

  if (labelEl) labelEl.textContent = `Macintosh HD · ${formatBytes(usage.free)} Available`;
  if (percentEl) percentEl.textContent = `${pct.toFixed(0)}%`;
  if (statUsedEl) statUsedEl.textContent = formatBytes(usage.used);
  if (statFreeEl) statFreeEl.textContent = formatBytes(usage.free);
  if (statTotalEl) statTotalEl.textContent = formatBytes(usage.total);

  if (ringProgress) {
    const circumference = 2 * Math.PI * 52; // r=52 (~326.73)
    const offset = circumference - (pct / 100) * circumference;
    ringProgress.style.strokeDashoffset = `${offset}`;
  }
}

// ─── Action Bar & Selection Management ─────────────────────────

function updateSelectionUI() {
  const countEl = document.getElementById("selected-count");
  const sizeEl = document.getElementById("selected-size");
  const cleanBtn = document.getElementById("btn-clean") as HTMLButtonElement | null;
  const cleanBtnText = document.getElementById("btn-clean-text");
  const actionBar = document.getElementById("action-bar");

  if (!currentScanResult) return;

  const allItems = [...currentScanResult.safe_items, ...currentScanResult.review_items];
  const selectedItems = allItems.filter((item) => selectedItemIds.has(item.id));
  const totalSelectedSize = selectedItems.reduce((acc, item) => acc + item.size, 0);

  if (countEl) {
    countEl.textContent = `${selectedItems.length} ${selectedItems.length === 1 ? "item" : "items"} selected`;
  }
  if (sizeEl) {
    sizeEl.textContent = formatBytes(totalSelectedSize);
  }

  if (cleanBtn) {
    cleanBtn.disabled = selectedItems.length === 0 || isDeleting;
  }
  if (cleanBtnText) {
    cleanBtnText.textContent = isDeleting
      ? "Cleaning..."
      : `Clean Selected (${formatBytes(totalSelectedSize)})`;
  }

  if (actionBar) {
    if (selectedItems.length > 0) {
      actionBar.classList.remove("hidden");
    } else {
      actionBar.classList.add("hidden");
    }
  }

  // Update item card selected classes
  document.querySelectorAll(".cleanup-item").forEach((el) => {
    const id = el.getAttribute("data-id");
    if (id && selectedItemIds.has(id)) {
      el.classList.add("selected");
    } else {
      el.classList.remove("selected");
    }
  });

  // Update Select All buttons state
  updateSelectAllButtons();
}

function updateSelectAllButtons() {
  if (!currentScanResult) return;

  const btnSafe = document.getElementById("btn-select-all-safe");
  const btnReview = document.getElementById("btn-select-all-review");

  if (btnSafe && currentScanResult.safe_items.length > 0) {
    const allSafeSelected = currentScanResult.safe_items.every((i) => selectedItemIds.has(i.id));
    btnSafe.textContent = allSafeSelected ? "Deselect All" : "Select All";
  }

  if (btnReview && currentScanResult.review_items.length > 0) {
    const allReviewSelected = currentScanResult.review_items.every((i) => selectedItemIds.has(i.id));
    btnReview.textContent = allReviewSelected ? "Deselect All" : "Select All";
  }
}

// ─── Render Results List ───────────────────────────────────────

function createItemElement(item: CleanupItem): HTMLElement {
  const div = document.createElement("div");
  div.className = "cleanup-item";
  div.setAttribute("data-id", item.id);
  if (selectedItemIds.has(item.id)) {
    div.classList.add("selected");
  }

  div.innerHTML = `
    <div class="item-checkbox"></div>
    <div class="item-icon">${item.icon}</div>
    <div class="item-info">
      <div class="item-name">${item.name}</div>
      <div class="item-desc" title="${item.description} (${item.path})">${item.description}</div>
    </div>
    <div class="item-meta">
      <div class="item-size">${formatBytes(item.size)}</div>
      <div class="item-files">${item.file_count.toLocaleString()} files</div>
    </div>
  `;

  div.addEventListener("click", () => {
    if (selectedItemIds.has(item.id)) {
      selectedItemIds.delete(item.id);
    } else {
      selectedItemIds.add(item.id);
    }
    updateSelectionUI();
  });

  return div;
}

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

  // Auto-select all safe items by default
  selectedItemIds.clear();
  result.safe_items.forEach((item) => selectedItemIds.add(item.id));

  // Populate Safe items
  if (result.safe_items.length === 0) {
    safeList.innerHTML = `
      <div class="empty-state">
        <span class="empty-icon">🎉</span>
        <span class="empty-text">No system caches or logs need cleaning</span>
      </div>
    `;
  } else {
    result.safe_items.forEach((item) => {
      safeList.appendChild(createItemElement(item));
    });
  }

  // Populate Review items
  if (result.review_items.length === 0) {
    reviewList.innerHTML = `
      <div class="empty-state">
        <span class="empty-icon">✨</span>
        <span class="empty-text">No developer or app caches found</span>
      </div>
    `;
  } else {
    result.review_items.forEach((item) => {
      reviewList.appendChild(createItemElement(item));
    });
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

  // Animate status messages during scanning
  const statusTexts = [
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
    currentScanResult = result;
    updateDiskUsageUI(result.disk_usage);
    renderResults(result);
    showToast(`Scan complete: Found ${formatBytes(result.total_safe_size + result.total_review_size)} reclaimable space`, "info");
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
  if (!currentScanResult || isDeleting || selectedItemIds.size === 0) return;

  const allItems = [...currentScanResult.safe_items, ...currentScanResult.review_items];
  const selectedItems = allItems.filter((item) => selectedItemIds.has(item.id));
  const hasReviewItems = selectedItems.some((item) => item.safety_level === "NeedsReview");

  // If user selected items from Needs Review, ask for explicit confirmation
  if (hasReviewItems) {
    const reviewItemNames = selectedItems
      .filter((item) => item.safety_level === "NeedsReview")
      .map((i) => `• ${i.name} (${formatBytes(i.size)})`)
      .join("<br/>");

    const confirmed = await showConfirmDialog(
      "Confirm Review Items Cleanup",
      `You selected items that require review:<br/><br/>${reviewItemNames}<br/><br/>Are you sure you want to clean these items? (Files will be moved to macOS Trash)`
    );

    if (!confirmed) return;
  }

  isDeleting = true;
  updateSelectionUI();

  const paths = selectedItems.map((item) => item.path);

  try {
    const result = await invoke<DeleteResult>("delete_items", { paths });
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

// ─── Initial Load & Event Listeners ────────────────────────────

window.addEventListener("DOMContentLoaded", async () => {
  // Fetch initial disk usage
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
    if (!currentScanResult) return;
    const allSafeSelected = currentScanResult.safe_items.every((i) => selectedItemIds.has(i.id));
    currentScanResult.safe_items.forEach((item) => {
      if (allSafeSelected) {
        selectedItemIds.delete(item.id);
      } else {
        selectedItemIds.add(item.id);
      }
    });
    updateSelectionUI();
  });

  // Select All Review
  document.getElementById("btn-select-all-review")?.addEventListener("click", () => {
    if (!currentScanResult) return;
    const allReviewSelected = currentScanResult.review_items.every((i) => selectedItemIds.has(i.id));
    currentScanResult.review_items.forEach((item) => {
      if (allReviewSelected) {
        selectedItemIds.delete(item.id);
      } else {
        selectedItemIds.add(item.id);
      }
    });
    updateSelectionUI();
  });

  // Auto trigger initial scan on startup
  runScan();
});
