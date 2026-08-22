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
    }
}

// ─── Tauri Commands ────────────────────────────────────────────

#[tauri::command]
fn get_disk_usage() -> Result<DiskUsage, String> {
    // Use statvfs on macOS to get disk usage for the root filesystem
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

        // Calculate size before deleting
        let (size, _) = get_dir_size(&path);

        // For the Trash folder, permanently delete contents
        if path_str.contains(".Trash") {
            match delete_dir_contents(&path) {
                Ok(count) => {
                    deleted_count += count;
                    freed_bytes += size;
                }
                Err(e) => errors.push(format!("{}: {}", path_str, e)),
            }
        } else {
            // For other items, delete the contents of the directory but keep the dir
            match delete_dir_contents(&path) {
                Ok(count) => {
                    deleted_count += count;
                    freed_bytes += size;
                }
                Err(e) => errors.push(format!("{}: {}", path_str, e)),
            }
        }
    }

    Ok(DeleteResult {
        success: errors.is_empty(),
        deleted_count,
        freed_bytes,
        errors,
    })
}

fn delete_dir_contents(path: &Path) -> Result<u32, String> {
    let mut count: u32 = 0;
    
    if !path.is_dir() {
        let _ = fs::remove_file(path);
        return Ok(1);
    }

    let entries: Vec<_> = fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .collect();

    for entry in entries {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if fs::remove_dir_all(&entry_path).is_ok() {
                count += 1;
            }
        } else {
            if fs::remove_file(&entry_path).is_ok() {
                count += 1;
            }
        }
    }

    Ok(count)
}

// ─── App Entry ─────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_disk_usage,
            scan_system,
            delete_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
