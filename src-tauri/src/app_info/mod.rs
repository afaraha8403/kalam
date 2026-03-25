//! Resolve friendly application names and icons from executable paths / process names.

mod platform;

/// Result of resolving metadata for a foreground app at dictation time.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// Normalized process filename (e.g. `chrome.exe`, `cursor`).
    #[allow(dead_code)]
    pub process_name: String,
    /// Human-readable name (e.g. "Google Chrome").
    pub display_name: String,
    /// PNG bytes for a small icon, when the OS provides one.
    pub icon_png: Option<Vec<u8>>,
}

/// Lowercase filename stem used as `applications.process_name` and for cache lookups.
pub fn normalize_process_name(name: &str) -> String {
    std::path::Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| name.to_lowercase())
}

/// Fallback label when OS metadata is missing (e.g. `notepad.exe` → `Notepad`).
pub fn capitalize_process_name(name: &str) -> String {
    let base = name.trim();
    let base = base
        .strip_suffix(".exe")
        .unwrap_or(base)
        .strip_suffix(".EXE")
        .unwrap_or(base);
    base.replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resolve display name and icon using the best path hint (full exe path preferred).
pub fn resolve(exe_path_or_name: &str) -> Option<AppInfo> {
    let trimmed = exe_path_or_name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let process_name = normalize_process_name(trimmed);
    let path = std::path::Path::new(trimmed);
    let looks_like_fs_path = path.is_absolute()
        || trimmed.contains(std::path::MAIN_SEPARATOR)
        || (cfg!(windows) && trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':');

    let (display_name, icon_png) = if looks_like_fs_path && path.exists() {
        let dn = platform::resolve_display_name(trimmed).unwrap_or_else(|| {
            capitalize_process_name(&process_name)
        });
        let icon = platform::resolve_icon(trimmed);
        (dn, icon)
    } else {
        // No usable path: only derive a fallback title; icon may be filled later via DB cache.
        let dn = platform::resolve_display_name_from_process_only(trimmed, &process_name)
            .unwrap_or_else(|| capitalize_process_name(&process_name));
        (dn, None)
    };

    Some(AppInfo {
        process_name,
        display_name,
        icon_png,
    })
}
