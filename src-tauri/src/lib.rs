use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ─── Data Types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyLevel {
    Safe,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubCleanupItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub file_count: u64,
    pub is_dir: bool,
    pub has_children: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<SubCleanupItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub category: String,
    pub icon: String,
    pub safety_level: SafetyLevel,
    pub description: String,
    pub file_count: u64,
    pub exists: bool,
    pub children: Vec<SubCleanupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub percentage_used: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub disk_usage: DiskUsage,
    pub safe_items: Vec<CleanupItem>,
    pub review_items: Vec<CleanupItem>,
    pub total_safe_size: u64,
    pub total_review_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResult {
    pub success: bool,
    pub deleted_count: u32,
    pub freed_bytes: u64,
    pub errors: Vec<String>,
}

// ─── Helper Functions ──────────────────────────────────────────

fn get_dir_size(path: &Path) -> (u64, u64) {
    let mut total_size: u64 = 0;
    let mut file_count: u64 = 0;
    
    if !path.exists() {
        return (0, 0);
    }

    if path.is_file() {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        return (size, 1);
    }

    for entry in WalkDir::new(path)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }

    (total_size, file_count)
}

fn expand_home(path_str: &str) -> PathBuf {
    if path_str.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path_str[2..]);
        }
    }
    PathBuf::from(path_str)
}

fn scan_children_direct(path: &Path) -> Vec<SubCleanupItem> {
    if !path.exists() || !path.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };

    let mut items: Vec<SubCleanupItem> = Vec::new();

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name == ".DS_Store" || file_name == ".localized" {
            continue;
        }

        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let (size, file_count) = if is_dir {
            get_dir_size(&entry_path)
        } else {
            let s = entry.metadata().map(|m| m.len()).unwrap_or(0);
            (s, 1)
        };

        if size == 0 && file_count == 0 {
            continue;
        }

        let has_children = if is_dir {
            fs::read_dir(&entry_path)
                .map(|mut r| r.next().is_some())
                .unwrap_or(false)
        } else {
            false
        };

        items.push(SubCleanupItem {
            id: entry_path.to_string_lossy().to_string(),
            name: file_name,
            path: entry_path.to_string_lossy().to_string(),
            size,
            file_count,
            is_dir,
            has_children,
            children: None,
        });
    }

    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

fn scan_location(
    id: &str,
    name: &str,
    path_str: &str,
    category: &str,
    icon: &str,
    safety_level: SafetyLevel,
    description: &str,
) -> CleanupItem {
    let path = expand_home(path_str);
    let exists = path.exists();
    let (size, file_count) = if exists {
        get_dir_size(&path)
    } else {
        (0, 0)
    };

    let children = if exists && path.is_dir() {
        scan_children_direct(&path)
    } else {
        Vec::new()
    };

    CleanupItem {
        id: id.to_string(),
        name: name.to_string(),
        path: path.to_string_lossy().to_string(),
        size,
        category: category.to_string(),
        icon: icon.to_string(),
        safety_level,
        description: description.to_string(),
        file_count,
        exists,
        children,
    }
}

fn is_category_root(path_str: &str) -> bool {
    let roots = [
        "/Library/Caches",
        "/Library/Logs",
        "/.Trash",
        "/Library/Developer/Xcode/DerivedData",
        "/Library/Developer/Xcode/Archives",
        "/Library/Developer/CoreSimulator/Devices",
        "/Downloads",
        "/.npm/_cacache",
        "/.gradle/caches",
        "/Library/Application Support/CrashReporter",
    ];
    roots.iter().any(|r| path_str.ends_with(r))
}

fn delete_dir_contents_resilient(path: &Path) -> (u64, u32, Option<String>) {
    let mut freed_bytes: u64 = 0;
    let mut deleted_count: u32 = 0;
    let mut last_err = None;

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => return (0, 0, Some(e.to_string())),
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                let (sub_size, sub_files) = get_dir_size(&entry_path);
                if fs::remove_dir_all(&entry_path).is_ok() {
                    freed_bytes += sub_size;
                    deleted_count += sub_files.max(1) as u32;
                } else {
                    let (f, c, err) = delete_dir_contents_resilient(&entry_path);
                    freed_bytes += f;
                    deleted_count += c;
                    if err.is_some() {
                        last_err = err;
                    }
                    let _ = fs::remove_dir(&entry_path);
                }
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if fs::remove_file(&entry_path).is_ok() {
                    freed_bytes += size;
                    deleted_count += 1;
                }
            }
        }
    }

    (freed_bytes, deleted_count, last_err)
}

fn delete_target(path: &Path) -> (u64, u32, Option<String>) {
    let path_str = path.to_string_lossy();
    if is_category_root(&path_str) {
        delete_dir_contents_resilient(path)
    } else if path.is_file() || path.is_symlink() {
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        if fs::remove_file(path).is_ok() {
            (size, 1, None)
        } else {
            (0, 0, Some(format!("Could not remove file: {}", path_str)))
        }
    } else if path.is_dir() {
        let (size, files) = get_dir_size(path);
        if fs::remove_dir_all(path).is_ok() {
            (size, files.max(1) as u32, None)
        } else {
            let (freed, count, err) = delete_dir_contents_resilient(path);
            let _ = fs::remove_dir(path);
            (freed, count, err)
        }
    } else {
        (0, 0, None)
    }
}

// ─── Tauri Commands ────────────────────────────────────────────

#[tauri::command]
fn get_disk_usage() -> Result<DiskUsage, String> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::mem;

        let path = CString::new("/").map_err(|e| e.to_string())?;
        unsafe {
            let mut stat: libc::statvfs = mem::zeroed();
            if libc::statvfs(path.as_ptr(), &mut stat) == 0 {
                let block_size = stat.f_frsize as u64;
                let total = stat.f_blocks as u64 * block_size;
                let free = stat.f_bavail as u64 * block_size;
                let used = total - free;
                let percentage_used = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                return Ok(DiskUsage {
                    total,
                    used,
                    free,
                    percentage_used,
                });
            }
        }
    }
    
    Err("Could not get disk usage".to_string())
}

#[tauri::command]
async fn scan_sub_items(path: String) -> Result<Vec<SubCleanupItem>, String> {
    let p = expand_home(&path);
    if !p.exists() {
        return Ok(Vec::new());
    }
    Ok(scan_children_direct(&p))
}

#[tauri::command]
async fn scan_system() -> Result<ScanResult, String> {
    let disk_usage = get_disk_usage().unwrap_or(DiskUsage {
        total: 0,
        used: 0,
        free: 0,
        percentage_used: 0.0,
    });

    // ── Safe to delete items ──
    let safe_items: Vec<CleanupItem> = vec![
        scan_location(
            "user_caches",
            "User Caches",
            "~/Library/Caches",
            "Caches",
            "📦",
            SafetyLevel::Safe,
            "Application cache files. Apps will recreate them when needed.",
        ),
        scan_location(
            "app_logs",
            "Application Logs",
            "~/Library/Logs",
            "Logs",
            "📋",
            SafetyLevel::Safe,
            "App log files. Generally not needed unless debugging.",
        ),
        scan_location(
            "crash_reports",
            "Crash Reports",
            "~/Library/Application Support/CrashReporter",
            "Logs",
            "💥",
            SafetyLevel::Safe,
            "Old crash report data. Safe to remove if not debugging.",
        ),
        scan_location(
            "trash",
            "Trash",
            "~/.Trash",
            "Trash",
            "🗑️",
            SafetyLevel::Safe,
            "Files in your Trash bin. Permanently delete to reclaim space.",
        ),
        scan_location(
            "safari_cache",
            "Safari Cache",
            "~/Library/Caches/com.apple.Safari",
            "Browser",
            "🌐",
            SafetyLevel::Safe,
            "Safari browser cache. Will be rebuilt as you browse.",
        ),
    ]
    .into_iter()
    .filter(|item| item.exists && item.size > 0)
    .collect();

    // ── Needs review items ──
    let review_items: Vec<CleanupItem> = vec![
        scan_location(
            "xcode_derived",
            "Xcode DerivedData",
            "~/Library/Developer/Xcode/DerivedData",
            "Developer",
            "🔨",
            SafetyLevel::NeedsReview,
            "Xcode build artifacts. Can be rebuilt but may slow down builds.",
        ),
        scan_location(
            "xcode_archives",
            "Xcode Archives",
            "~/Library/Developer/Xcode/Archives",
            "Developer",
            "📁",
            SafetyLevel::NeedsReview,
            "Archived builds. Remove old ones you no longer need.",
        ),
        scan_location(
            "ios_simulators",
            "iOS Simulators",
            "~/Library/Developer/CoreSimulator/Devices",
            "Developer",
            "📱",
            SafetyLevel::NeedsReview,
            "iOS Simulator data. Can be re-downloaded from Xcode.",
        ),
        scan_location(
            "npm_cache",
            "npm Cache",
            "~/.npm/_cacache",
            "Developer",
            "📦",
            SafetyLevel::NeedsReview,
            "npm package cache. Will be rebuilt on next install.",
        ),
        scan_location(
            "yarn_cache",
            "Yarn Cache",
            "~/Library/Caches/Yarn",
            "Developer",
            "🧶",
            SafetyLevel::NeedsReview,
            "Yarn package cache. Can be rebuilt with yarn install.",
        ),
        scan_location(
            "cocoapods_cache",
            "CocoaPods Cache",
            "~/Library/Caches/CocoaPods",
            "Developer",
            "🫛",
            SafetyLevel::NeedsReview,
            "CocoaPods dependency cache. Will re-download on pod install.",
        ),
        scan_location(
            "gradle_cache",
            "Gradle Caches",
            "~/.gradle/caches",
            "Developer",
            "🐘",
            SafetyLevel::NeedsReview,
            "Gradle build cache for Android/Java projects.",
        ),
        scan_location(
            "docker_data",
            "Docker Data",
            "~/Library/Containers/com.docker.docker",
            "Apps",
            "🐳",
            SafetyLevel::NeedsReview,
            "Docker images, containers, and volumes. Review before deleting.",
        ),
        scan_location(
            "vscode_cache",
            "VS Code Cache",
            "~/Library/Application Support/Code/Cache",
            "Apps",
            "💻",
            SafetyLevel::NeedsReview,
            "VS Code editor cache files.",
        ),
        scan_location(
            "spotify_cache",
            "Spotify Cache",
            "~/Library/Application Support/Spotify/PersistentCache",
            "Apps",
            "🎵",
            SafetyLevel::NeedsReview,
            "Spotify offline music cache.",
        ),
        scan_location(
            "pip_cache",
            "pip Cache",
            "~/Library/Caches/pip",
            "Developer",
            "🐍",
            SafetyLevel::NeedsReview,
            "Python pip download cache.",
        ),
        scan_location(
            "homebrew_cache",
            "Homebrew Cache",
            "~/Library/Caches/Homebrew",
            "Developer",
            "🍺",
            SafetyLevel::NeedsReview,
            "Homebrew downloaded packages. Run 'brew cleanup' instead for better control.",
        ),
        scan_location(
            "old_downloads",
            "Downloads Folder",
            "~/Downloads",
            "User Data",
            "⬇️",
            SafetyLevel::NeedsReview,
            "Your Downloads folder. Review files before deleting.",
        ),
    ]
    .into_iter()
    .filter(|item| item.exists && item.size > 0)
    .collect();

    let total_safe_size: u64 = safe_items.iter().map(|i| i.size).sum();
    let total_review_size: u64 = review_items.iter().map(|i| i.size).sum();

    Ok(ScanResult {
        disk_usage,
        safe_items,
        review_items,
        total_safe_size,
        total_review_size,
    })
}

#[tauri::command]
async fn delete_items(paths: Vec<String>) -> Result<DeleteResult, String> {
    let mut deleted_count: u32 = 0;
    let mut freed_bytes: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for path_str in &paths {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            continue;
        }

        let (freed, count, err) = delete_target(&path);
        freed_bytes += freed;
        deleted_count += count;
        if let Some(e) = err {
            errors.push(format!("{}: {}", path_str, e));
        }
    }

    Ok(DeleteResult {
        success: errors.is_empty(),
        deleted_count,
        freed_bytes,
        errors,
    })
}

// ─── App Entry ─────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_disk_usage,
            scan_system,
            scan_sub_items,
            delete_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
