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
    pub safety_tip: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFile {
    pub name: String,
    pub path: String,
    pub size: u64,
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
    safety_tip: &str,
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
        safety_tip: safety_tip.to_string(),
        file_count,
        exists,
        children,
    }
}

fn is_category_root(path_str: &str) -> bool {
    // Normalize path separators and case so the same rules work on both
    // macOS ("/Users/.../Library/...") and Windows ("C:\\Users\\...").
    let normalized = path_str.replace('\\', "/").to_lowercase();
    let roots = [
        // ── macOS roots ──
        "/library/caches",
        "/library/logs",
        "/.trash",
        "/library/developer/xcode/deriveddata",
        "/library/developer/xcode/archives",
        "/library/developer/coresimulator/devices",
        "/downloads",
        "/.npm",
        "/.npm/_cacache",
        "/.gradle/caches",
        "/library/application support/crashreporter",
        "/library/developer/xcode/userdata/ib support",
        "/library/developer/xcode/userdata/ib%20support",
        "/library/developer/xcode/ios devicesupport",
        "/library/developer/xcode/documentationcache",
        "/.cache/uv",
        "/.cache/huggingface",
        "/.cache/profileforge",
        "/.cache/solana",
        "/.cargo/registry/cache",
        "/library/caches/google",
        "/library/caches/pip",
        "/library/caches/homebrew",
        "/library/caches/dev.kiro.desktop.shipit",
        "/library/caches/com.github.githubclient.shipit",
        "/library/caches/dotslash",
        "/.meteor/packages",
        "/library/application support/code/cache",
        "/library/application support/zed/cache",
        // ── Windows roots ──
        "/appdata/local/temp",
        "/windows/temp",
        "/appdata/local/microsoft/windows/inetcache",
        "/appdata/local/microsoft/windows/wer",
        "/appdata/local/crashdumps",
        "/appdata/local/d3dscache",
        "/appdata/local/npm-cache",
        "/appdata/local/pip/cache",
        "/appdata/local/uv/cache",
        "/appdata/local/google/chrome/user data/default/cache",
        "/appdata/local/microsoft/edge/user data/default/cache",
        "/appdata/roaming/code/cache",
        "/appdata/roaming/spotify/storage",
        "/appdata/local/yarn/cache",
        "/appdata/local/docker",
        "/.nuget/packages",
        "/$recycle.bin",
        "/windows/softwaredistribution/download",
    ];
    roots.iter().any(|r| normalized.ends_with(r))
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
            let mut stat: libc::statfs = mem::zeroed();
            if libc::statfs(path.as_ptr(), &mut stat) == 0 {
                let block_size = stat.f_bsize as u64;
                let total = stat.f_blocks * block_size;
                let free = stat.f_bavail * block_size;
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
    
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        let root: Vec<u16> = OsStr::new(&format!("{}\\", drive))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut total_bytes: u64 = 0;
        let mut available_bytes: u64 = 0;
        let mut total_free_bytes: u64 = 0;

        unsafe {
            let ok = windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                root.as_ptr(),
                &mut available_bytes,
                &mut total_bytes,
                &mut total_free_bytes,
            );
            if ok != 0 {
                let total = total_bytes;
                let free = available_bytes;
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

/// Root node for the size-sorted directory tree (the user's home folder).
#[tauri::command]
async fn get_tree_root() -> Result<SubCleanupItem, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not resolve home directory".to_string())?;
    let (size, file_count) = get_dir_size(&home);
    let name = home
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Home".to_string());
    Ok(SubCleanupItem {
        id: home.to_string_lossy().to_string(),
        name,
        path: home.to_string_lossy().to_string(),
        size,
        file_count,
        is_dir: true,
        has_children: true,
        children: None,
    })
}

/// Scan the home directory for the largest individual files.
#[tauri::command]
async fn scan_large_files(min_size_mb: u64, limit: usize) -> Result<Vec<LargeFile>, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not resolve home directory".to_string())?;
    let min_bytes = min_size_mb.saturating_mul(1024 * 1024);
    let limit = if limit == 0 { 200 } else { limit };

    let mut files: Vec<LargeFile> = Vec::new();

    for entry in WalkDir::new(&home)
        .max_depth(24)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden dotfiles/dirs and the recycle bin to avoid noise.
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && name.starts_with('.') {
                return false;
            }
            if name == "$RECYCLE.BIN" || name == "System Volume Information" {
                return false;
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if size < min_bytes {
            continue;
        }
        files.push(LargeFile {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            size,
        });
    }

    files.sort_by(|a, b| b.size.cmp(&a.size));
    files.truncate(limit);
    Ok(files)
}

/// Reveal a file/folder in the native file manager (Finder / Explorer).
#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    let p = expand_home(&path);
    if !p.exists() {
        return Err("Path does not exist".to_string());
    }
    let target = p.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", target))
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = target;
        Err("Reveal is not supported on this platform".to_string())
    }
}

#[tauri::command]
#[cfg(target_os = "macos")]
async fn scan_system() -> Result<ScanResult, String> {
    let disk_usage = get_disk_usage().unwrap_or(DiskUsage {
        total: 0,
        used: 0,
        free: 0,
        percentage_used: 0.0,
    });

    // ── Safe to delete items (100% An toàn - Tự động tái tạo khi cần) ──
    let safe_items: Vec<CleanupItem> = vec![
        scan_location(
            "user_caches",
            "User Caches",
            "~/Library/Caches",
            "Caches",
            "📦",
            SafetyLevel::Safe,
            "Bộ nhớ đệm ứng dụng. Ứng dụng sẽ tự động tạo lại tệp đệm khi khởi chạy.",
            "🟢 100% An toàn. Không mất dữ liệu cá nhân hay tài khoản, hệ thống & app tự tạo lại khi cần.",
        ),
        scan_location(
            "xcode_ib_support",
            "Xcode Interface Builder Cache",
            "~/Library/Developer/Xcode/UserData/IB Support",
            "Developer",
            "🎨",
            SafetyLevel::Safe,
            "Bộ nhớ đệm render trực quan Storyboard/XIB của Xcode (thường phình to 10 - 30 GB).",
            "🟢 100% An toàn. Xcode tự sinh lại cache render khi mở project, không ảnh hưởng code hay build.",
        ),
        scan_location(
            "uv_cache",
            "Python uv Package Cache",
            "~/.cache/uv",
            "Developer",
            "⚡",
            SafetyLevel::Safe,
            "Bộ nhớ đệm tải về các gói thư viện Python của công cụ uv.",
            "🟢 100% An toàn. Các môi trường venv đã cài đặt vẫn nguyên vẹn. uv sẽ tự tải lại khi install gói mới.",
        ),
        scan_location(
            "chrome_cache",
            "Google Chrome Cache",
            "~/Library/Caches/Google",
            "Browser",
            "🌐",
            SafetyLevel::Safe,
            "Bộ nhớ đệm web và hình ảnh của Google Chrome.",
            "🟢 100% An toàn. Không làm mất bookmark, mật khẩu đã lưu hay phiên đăng nhập.",
        ),
        scan_location(
            "safari_cache",
            "Safari Cache",
            "~/Library/Caches/com.apple.Safari",
            "Browser",
            "🧭",
            SafetyLevel::Safe,
            "Bộ nhớ đệm duyệt web của Safari.",
            "🟢 100% An toàn. Safari sẽ tự tạo lại khi bạn lướt web, không ảnh hưởng mật khẩu hay lịch sử.",
        ),
        scan_location(
            "app_logs",
            "Application Logs",
            "~/Library/Logs",
            "Logs",
            "📋",
            SafetyLevel::Safe,
            "Tệp ghi nhật ký hoạt động cũ của các ứng dụng.",
            "🟢 100% An toàn. Chỉ dùng khi lập trình viên cần xem lại lỗi cũ, hoàn toàn không cần cho app chạy bình thường.",
        ),
        scan_location(
            "crash_reports",
            "Crash Reports",
            "~/Library/Application Support/CrashReporter",
            "Logs",
            "💥",
            SafetyLevel::Safe,
            "Dữ liệu báo cáo sự cố (crash dump) cũ của macOS.",
            "🟢 100% An toàn. Là các báo cáo sau khi app văng, an toàn xóa bỏ để lấy lại dung lượng.",
        ),
        scan_location(
            "trash",
            "Trash Bin",
            "~/.Trash",
            "Trash",
            "🗑️",
            SafetyLevel::Safe,
            "Tệp trong Thùng rác chưa được dọn vĩnh viễn.",
            "🟢 An toàn để dọn sạch hoàn toàn các tệp bạn đã chủ động xóa vào Thùng rác.",
        ),
        scan_location(
            "xcode_doc_cache",
            "Xcode Documentation Cache",
            "~/Library/Developer/Xcode/DocumentationCache",
            "Developer",
            "📚",
            SafetyLevel::Safe,
            "Bộ nhớ đệm tài liệu lập trình offline của Xcode.",
            "🟢 100% An toàn. Xcode sẽ tự động tải lại khi bạn tra cứu tài liệu mới.",
        ),
        scan_location(
            "npm_cache",
            "npm Package Cache",
            "~/.npm",
            "Developer",
            "📦",
            SafetyLevel::Safe,
            "Bộ nhớ đệm gói Node.js npm tải từ registry.",
            "🟢 100% An toàn. npm sẽ tự động tải lại gói khi bạn chạy lệnh npm install.",
        ),
        scan_location(
            "cargo_cache",
            "Cargo Registry Cache",
            "~/.cargo/registry/cache",
            "Developer",
            "🦀",
            SafetyLevel::Safe,
            "Tệp nén .crate đã tải về của Rust Cargo.",
            "🟢 100% An toàn. Mã nguồn crate đã giải nén trong project vẫn nguyên vẹn. Cargo tự tải lại khi cập nhật.",
        ),
        scan_location(
            "pip_cache",
            "pip Download Cache",
            "~/Library/Caches/pip",
            "Developer",
            "🐍",
            SafetyLevel::Safe,
            "Bộ nhớ đệm tải gói của Python pip.",
            "🟢 100% An toàn. Thư viện Python đã cài đặt trong máy không bị ảnh hưởng.",
        ),
        scan_location(
            "app_shipit_cache",
            "Desktop App Update Cache",
            "~/Library/Caches/dev.kiro.desktop.ShipIt",
            "Apps",
            "🚀",
            SafetyLevel::Safe,
            "Tệp cài đặt cập nhật cũ của Kiro Desktop và Electron apps.",
            "🟢 100% An toàn. Ứng dụng đã được cập nhật thành công, bộ cài tải về cũ không còn tác dụng.",
        ),
        scan_location(
            "dotslash_cache",
            "DotSlash & CLI Cache",
            "~/Library/Caches/dotslash",
            "Developer",
            "⚙️",
            SafetyLevel::Safe,
            "Bộ nhớ đệm thực thi của công cụ DotSlash.",
            "🟢 100% An toàn. Sẽ tự động tải lại binary khi cần chạy script.",
        ),
        scan_location(
            "solana_cache",
            "Solana CLI Cache",
            "~/.cache/solana",
            "Developer",
            "🪙",
            SafetyLevel::Safe,
            "Bộ nhớ đệm ledger và công cụ Solana CLI.",
            "🟢 100% An toàn. Không làm mất private key hay tài khoản ví.",
        ),
    ]
    .into_iter()
    .filter(|item| item.exists && item.size > 0)
    .collect();

    // ── Needs review items (Cần xem xét - Tải/Build lại khi dùng) ──
    let review_items: Vec<CleanupItem> = vec![
        scan_location(
            "ios_device_support",
            "iOS DeviceSupport Symbols",
            "~/Library/Developer/Xcode/iOS DeviceSupport",
            "Developer",
            "📱",
            SafetyLevel::NeedsReview,
            "File biểu tượng gỡ lỗi (.symbols) của các iPhone đã cắm vào máy Mac (thường 10 - 30 GB).",
            "🟡 Cần xem xét: KHÔNG ảnh hưởng build app hay chạy bot Farm. Xcode chỉ dùng khi soi crash Instruments (Xcode sẽ tự trích xuất lại khi cắm máy).",
        ),
        scan_location(
            "xcode_derived",
            "Xcode DerivedData",
            "~/Library/Developer/Xcode/DerivedData",
            "Developer",
            "🔨",
            SafetyLevel::NeedsReview,
            "Tệp build trung gian và chỉ mục mã nguồn của Xcode.",
            "🟡 Cần xem xét: Dự án sẽ build lại từ đầu ở lần mở tiếp theo (mất thêm vài phút biên dịch).",
        ),
        scan_location(
            "xcode_archives",
            "Xcode Archives",
            "~/Library/Developer/Xcode/Archives",
            "Developer",
            "📁",
            SafetyLevel::NeedsReview,
            "Các bản đóng gói ứng dụng (.xcarchive) đã xuất bản trước đây.",
            "🟡 Cần xem xét: Hãy mở rộng để chọn xóa các bản build cũ mà bạn không còn cần phân phối lên App Store.",
        ),
        scan_location(
            "ios_simulators",
            "iOS Simulators Data",
            "~/Library/Developer/CoreSimulator/Devices",
            "Developer",
            "📲",
            SafetyLevel::NeedsReview,
            "Dữ liệu ứng dụng và cache trong các máy ảo iOS Simulator.",
            "🟡 Cần xem xét: Xóa sẽ đặt lại máy ảo về mặc định xuất xưởng. Mở rộng để xóa từng máy ảo cụ thể.",
        ),
        scan_location(
            "huggingface_cache",
            "HuggingFace AI Models",
            "~/.cache/huggingface",
            "AI & Data",
            "🤗",
            SafetyLevel::NeedsReview,
            "Trọng số model AI đã tải từ HuggingFace Hub (thường nhiều GB).",
            "🟡 Cần xem xét: Code AI hoặc script Python sẽ cần tải lại model (qua internet) nếu bạn chạy lại tác vụ đó.",
        ),
        scan_location(
            "profileforge_cache",
            "ProfileForge Cache",
            "~/.cache/profileforge",
            "Developer",
            "👤",
            SafetyLevel::NeedsReview,
            "Dữ liệu hồ sơ giả lập của ProfileForge.",
            "🟡 Cần xem xét: Xóa sẽ làm mất các profile thử nghiệm tạm thời của ProfileForge.",
        ),
        scan_location(
            "yarn_cache",
            "Yarn Cache",
            "~/Library/Caches/Yarn",
            "Developer",
            "🧶",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm gói của Yarn. Có thể tải lại qua yarn install.",
            "🟡 Cần xem xét: Sẽ tải lại từ internet khi bạn chạy yarn install.",
        ),
        scan_location(
            "cocoapods_cache",
            "CocoaPods Cache",
            "~/Library/Caches/CocoaPods",
            "Developer",
            "🫛",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm thư viện CocoaPods cho iOS.",
            "🟡 Cần xem xét: Sẽ tải lại khi bạn chạy pod install.",
        ),
        scan_location(
            "gradle_cache",
            "Gradle Caches",
            "~/.gradle/caches",
            "Developer",
            "🐘",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm build của Gradle cho các dự án Android/Java.",
            "🟡 Cần xem xét: Lần build Android tiếp theo sẽ cần tải lại dependencies.",
        ),
        scan_location(
            "meteor_packages",
            "Meteor Framework Packages",
            "~/.meteor/packages",
            "Developer",
            "☄️",
            SafetyLevel::NeedsReview,
            "Các package của framework MeteorJS cũ cài đặt trước đây.",
            "🟡 Cần xem xét: Nếu bạn không còn chạy dự án Meteor nào thì hoàn toàn nên xóa để giải phóng dung lượng.",
        ),
        scan_location(
            "docker_data",
            "Docker Data",
            "~/Library/Containers/com.docker.docker",
            "Apps",
            "🐳",
            SafetyLevel::NeedsReview,
            "Dữ liệu images, containers và volumes của Docker.",
            "🟡 Cần xem xét: Xóa sẽ mất các container cục bộ và image đã build trong Docker.",
        ),
        scan_location(
            "vscode_cache",
            "VS Code Cache",
            "~/Library/Application Support/Code/Cache",
            "Apps",
            "💻",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm giao diện và tệp tạm của VS Code.",
            "🟡 Cần xem xét: Trình soạn thảo sẽ tự sinh lại cache mới, không làm mất extension hay cài đặt.",
        ),
        scan_location(
            "spotify_cache",
            "Spotify Offline Cache",
            "~/Library/Application Support/Spotify/PersistentCache",
            "Apps",
            "🎵",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm bài hát nghe gần đây hoặc tải offline trên Spotify.",
            "🟡 Cần xem xét: Không mất tài khoản hay playlist, chỉ cần tải lại bài hát khi muốn nghe offline.",
        ),
        scan_location(
            "homebrew_cache",
            "Homebrew Cache",
            "~/Library/Caches/Homebrew",
            "Developer",
            "🍺",
            SafetyLevel::NeedsReview,
            "Gói cài đặt đã tải về của Homebrew.",
            "🟡 Cần xem xét: Các phần mềm brew đã cài không bị ảnh hưởng. Bạn có thể xóa an toàn.",
        ),
        scan_location(
            "old_downloads",
            "Downloads Folder",
            "~/Downloads",
            "User Data",
            "⬇️",
            SafetyLevel::NeedsReview,
            "Thư mục Tải về cá nhân (file cài .dmg, .pkg, .zip...).",
            "🟡 Cần xem xét: Hãy mở rộng thư mục để chọn đúng các bộ cài cũ bạn muốn xóa, tránh xóa nhầm tài liệu quan trọng!",
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

// ─── Platform Auto-Detection ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub os_name: String,
    pub disk_label: String,
}

#[cfg(target_os = "macos")]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        os: "macos".to_string(),
        os_name: "macOS".to_string(),
        disk_label: "Macintosh HD".to_string(),
    }
}

#[cfg(target_os = "windows")]
fn platform_info() -> PlatformInfo {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    PlatformInfo {
        os: "windows".to_string(),
        os_name: "Windows".to_string(),
        disk_label: format!("Local Disk ({})", drive),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        os_name: std::env::consts::OS.to_string(),
        disk_label: "Disk".to_string(),
    }
}

#[tauri::command]
fn get_platform_info() -> PlatformInfo {
    platform_info()
}

// ─── Windows Scan (Tự động nhận diện máy Windows) ──────────────

#[tauri::command]
#[cfg(target_os = "windows")]
async fn scan_system() -> Result<ScanResult, String> {
    let disk_usage = get_disk_usage().unwrap_or(DiskUsage {
        total: 0,
        used: 0,
        free: 0,
        percentage_used: 0.0,
    });

    // ── Safe to delete items (Windows - 100% An toàn - Tự động tái tạo khi cần) ──
    let safe_items: Vec<CleanupItem> = vec![
        scan_location(
            "user_temp",
            "User Temp Files",
            "~/AppData/Local/Temp",
            "Caches",
            "🧹",
            SafetyLevel::Safe,
            "Tệp tạm thời do các ứng dụng và Windows tạo ra trong thư mục Temp của người dùng.",
            "🟢 100% An toàn. Ứng dụng tự tạo lại tệp tạm khi cần, không mất dữ liệu cá nhân.",
        ),
        scan_location(
            "windows_temp",
            "Windows System Temp",
            "C:/Windows/Temp",
            "Caches",
            "🧹",
            SafetyLevel::Safe,
            "Tệp tạm thời của hệ thống Windows (thư mục Temp toàn cục).",
            "🟢 100% An toàn. Windows tự tạo lại khi cần. Một số tệp đang khóa có thể cần quyền Admin.",
        ),
        scan_location(
            "chrome_cache",
            "Google Chrome Cache",
            "~/AppData/Local/Google/Chrome/User Data/Default/Cache",
            "Browser",
            "🌐",
            SafetyLevel::Safe,
            "Bộ nhớ đệm web và hình ảnh của Google Chrome.",
            "🟢 100% An toàn. Không làm mất bookmark, mật khẩu đã lưu hay phiên đăng nhập.",
        ),
        scan_location(
            "edge_cache",
            "Microsoft Edge Cache",
            "~/AppData/Local/Microsoft/Edge/User Data/Default/Cache",
            "Browser",
            "🌐",
            SafetyLevel::Safe,
            "Bộ nhớ đệm web và hình ảnh của Microsoft Edge.",
            "🟢 100% An toàn. Không làm mất bookmark, mật khẩu đã lưu hay phiên đăng nhập.",
        ),
        scan_location(
            "inet_cache",
            "Windows INetCache",
            "~/AppData/Local/Microsoft/Windows/INetCache",
            "Browser",
            "🌐",
            SafetyLevel::Safe,
            "Bộ nhớ đệm HTTP của các thành phần hệ thống và ứng dụng Windows.",
            "🟢 100% An toàn. Hệ thống tự tạo lại tệp đệm khi cần.",
        ),
        scan_location(
            "d3dscache",
            "DirectX Shader Cache",
            "~/AppData/Local/D3DSCache",
            "Caches",
            "🎮",
            SafetyLevel::Safe,
            "Bộ nhớ đệm shader của DirectX dùng cho game và ứng dụng đồ họa.",
            "🟢 100% An toàn. Game/ứng dụng sẽ biên dịch lại shader khi chạy (chậm hơn lần đầu).",
        ),
        scan_location(
            "wer_reports",
            "Windows Error Reports",
            "~/AppData/Local/Microsoft/Windows/WER",
            "Logs",
            "💥",
            SafetyLevel::Safe,
            "Báo cáo lỗi (Error Reporting) mà Windows thu thập sau sự cố ứng dụng.",
            "🟢 100% An toàn. Chỉ là báo cáo gửi về Microsoft, không ảnh hưởng hệ thống.",
        ),
        scan_location(
            "crash_dumps",
            "User Crash Dumps",
            "~/AppData/Local/CrashDumps",
            "Logs",
            "💥",
            SafetyLevel::Safe,
            "Tệp dump bộ nhớ (crash dump) của ứng dụng bị văng.",
            "🟢 100% An toàn. Chỉ dùng để phân tích lỗi cũ, xóa bỏ hoàn toàn an toàn.",
        ),
        scan_location(
            "npm_cache",
            "npm Package Cache",
            "~/AppData/Local/npm-cache",
            "Developer",
            "📦",
            SafetyLevel::Safe,
            "Bộ nhớ đệm gói Node.js npm tải từ registry.",
            "🟢 100% An toàn. npm sẽ tự động tải lại gói khi bạn chạy npm install.",
        ),
        scan_location(
            "pip_cache",
            "pip Download Cache",
            "~/AppData/Local/pip/cache",
            "Developer",
            "🐍",
            SafetyLevel::Safe,
            "Bộ nhớ đệm tải gói của Python pip.",
            "🟢 100% An toàn. Thư viện Python đã cài đặt trong máy không bị ảnh hưởng.",
        ),
        scan_location(
            "uv_cache",
            "Python uv Package Cache",
            "~/AppData/Local/uv/cache",
            "Developer",
            "⚡",
            SafetyLevel::Safe,
            "Bộ nhớ đệm tải về các gói thư viện Python của công cụ uv.",
            "🟢 100% An toàn. Các môi trường venv đã cài đặt vẫn nguyên vẹn. uv sẽ tự tải lại khi cài gói mới.",
        ),
        scan_location(
            "cargo_cache",
            "Cargo Registry Cache",
            "~/.cargo/registry/cache",
            "Developer",
            "🦀",
            SafetyLevel::Safe,
            "Tệp nén .crate đã tải về của Rust Cargo.",
            "🟢 100% An toàn. Mã nguồn crate đã giải nén trong project vẫn nguyên vẹn. Cargo tự tải lại khi cập nhật.",
        ),
    ]
    .into_iter()
    .filter(|item| item.exists && item.size > 0)
    .collect();

    // ── Needs review items (Windows - Cần xem xét - Tải/Build lại khi dùng) ──
    let review_items: Vec<CleanupItem> = vec![
        scan_location(
            "downloads",
            "Downloads Folder",
            "~/Downloads",
            "User Data",
            "⬇️",
            SafetyLevel::NeedsReview,
            "Thư mục Tải về cá nhân (file cài .exe, .msi, .zip...).",
            "🟡 Cần xem xét: Hãy mở rộng thư mục để chọn đúng các bộ cài cũ bạn muốn xóa, tránh xóa nhầm tài liệu quan trọng!",
        ),
        scan_location(
            "recycle_bin",
            "Recycle Bin (Thùng rác)",
            "C:/$Recycle.Bin",
            "Trash",
            "🗑️",
            SafetyLevel::NeedsReview,
            "Tệp đã xóa trong Thùng rác của ổ đĩa hệ thống C:.",
            "🟡 Cần xem xét: Dọn sạch sẽ xóa vĩnh viễn toàn bộ nội dung Thùng rác. Có thể cần quyền Admin.",
        ),
        scan_location(
            "windows_update_cache",
            "Windows Update Download Cache",
            "C:/Windows/SoftwareDistribution/Download",
            "Apps",
            "🔄",
            SafetyLevel::NeedsReview,
            "Bộ cài các bản cập nhật Windows Update đã tải về.",
            "🟡 Cần xem xét: Cần quyền Admin. Windows sẽ tải lại bản cập nhật khi cần thiết.",
        ),
        scan_location(
            "windows_old",
            "Windows.old (Bản cài cũ)",
            "C:/Windows.old",
            "Apps",
            "🕰️",
            SafetyLevel::NeedsReview,
            "Toàn bộ bản cài đặt Windows cũ còn sót lại sau khi nâng cấp hệ điều hành (thường 20 - 50 GB).",
            "🟡 Cần xem xét: Sau khi xóa sẽ KHÔNG thể quay về phiên bản Windows cũ. Cần quyền Admin.",
        ),
        scan_location(
            "huggingface_cache",
            "HuggingFace AI Models",
            "~/.cache/huggingface",
            "AI & Data",
            "🤗",
            SafetyLevel::NeedsReview,
            "Trọng số model AI đã tải từ HuggingFace Hub (thường nhiều GB).",
            "🟡 Cần xem xét: Code AI hoặc script Python sẽ cần tải lại model (qua internet) nếu bạn chạy lại tác vụ đó.",
        ),
        scan_location(
            "gradle_cache",
            "Gradle Caches",
            "~/.gradle/caches",
            "Developer",
            "🐘",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm build của Gradle cho các dự án Android/Java.",
            "🟡 Cần xem xét: Lần build Android tiếp theo sẽ cần tải lại dependencies.",
        ),
        scan_location(
            "nuget_cache",
            "NuGet Package Cache",
            "~/.nuget/packages",
            "Developer",
            "📦",
            SafetyLevel::NeedsReview,
            "Thư viện NuGet đã tải về cho các dự án .NET.",
            "🟡 Cần xem xét: dotnet build sẽ tự tải lại gói từ nuget.org khi cần.",
        ),
        scan_location(
            "docker_data",
            "Docker Desktop Data",
            "~/AppData/Local/Docker",
            "Apps",
            "🐳",
            SafetyLevel::NeedsReview,
            "Dữ liệu images, containers và volumes của Docker Desktop.",
            "🟡 Cần xem xét: Xóa sẽ mất các container cục bộ và image đã build trong Docker.",
        ),
        scan_location(
            "vscode_cache",
            "VS Code Cache",
            "~/AppData/Roaming/Code/Cache",
            "Apps",
            "💻",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm giao diện và tệp tạm của VS Code.",
            "🟡 Cần xem xét: Trình soạn thảo sẽ tự sinh lại cache mới, không làm mất extension hay cài đặt.",
        ),
        scan_location(
            "spotify_cache",
            "Spotify Offline Cache",
            "~/AppData/Roaming/Spotify/Storage",
            "Apps",
            "🎵",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm bài hát nghe gần đây hoặc tải offline trên Spotify.",
            "🟡 Cần xem xét: Không mất tài khoản hay playlist, chỉ cần tải lại bài hát khi muốn nghe offline.",
        ),
        scan_location(
            "yarn_cache",
            "Yarn Cache",
            "~/AppData/Local/Yarn/Cache",
            "Developer",
            "🧶",
            SafetyLevel::NeedsReview,
            "Bộ nhớ đệm gói của Yarn. Có thể tải lại qua yarn install.",
            "🟡 Cần xem xét: Sẽ tải lại từ internet khi bạn chạy yarn install.",
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
            get_platform_info,
            get_disk_usage,
            scan_system,
            scan_sub_items,
            get_tree_root,
            scan_large_files,
            reveal_path,
            delete_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
