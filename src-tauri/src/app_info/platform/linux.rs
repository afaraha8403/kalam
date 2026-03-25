//! Linux: desktop entries for names; freedesktop icon cache for PNG paths.

use std::path::{Path, PathBuf};

use freedesktop_desktop_entry::DesktopEntry;
use freedesktop_icon_lookup::Cache;

use super::super::capitalize_process_name;

fn desktop_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        v.push(PathBuf::from(home).join(".local/share/applications"));
    }
    v
}

fn exe_basename_matches_exec_line(exe_path: &str, exec_line: &str) -> bool {
    let base = Path::new(exe_path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if base.is_empty() {
        return false;
    }
    for token in exec_line.split_whitespace() {
        if token.starts_with('-') || token.starts_with('%') {
            continue;
        }
        let t = token.trim_matches('"');
        if let Some(fname) = Path::new(t).file_name().and_then(|n| n.to_str()) {
            if fname.to_lowercase() == base {
                return true;
            }
        }
    }
    false
}

fn find_desktop_for_exe(exe_path: &str) -> Option<PathBuf> {
    let exe_lower = exe_path.to_lowercase();
    for dir in desktop_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if s.contains("Exec=") {
                        let exec_line = s
                            .lines()
                            .find(|l| l.starts_with("Exec="))
                            .map(|l| l.trim_start_matches("Exec="))?;
                        if exe_basename_matches_exec_line(exe_path, exec_line) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }
    // Secondary: path substring (flatpak / unusual layouts)
    for dir in desktop_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().map(|e| e == "desktop").unwrap_or(false) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if s.to_lowercase().contains(&exe_lower) {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

pub fn resolve_display_name(exe_path: &str) -> Option<String> {
    let desktop = find_desktop_for_exe(exe_path)?;
    let entry = DesktopEntry::from_path(&desktop, None::<&[&str]>).ok()?;
    entry.full_name::<&str>(&[]).map(|c| c.into_owned())
}

pub fn resolve_display_name_from_process_only(_original: &str, normalized: &str) -> Option<String> {
    Some(capitalize_process_name(normalized))
}

fn icon_png_from_name(icon_name: &str) -> Option<Vec<u8>> {
    let mut cache = Cache::new().ok()?;
    cache.load_default().ok()?;
    let _ = cache.load("Adwaita");
    let _ = cache.load("hicolor");
    let path = cache.lookup(icon_name, None::<&str>)?;
    std::fs::read(path).ok()
}

pub fn resolve_icon(exe_path: &str) -> Option<Vec<u8>> {
    if let Some(desktop_path) = find_desktop_for_exe(exe_path) {
        if let Ok(entry) = DesktopEntry::from_path(&desktop_path, None::<&[&str]>) {
            if let Some(icon) = entry.desktop_entry("Icon") {
                if Path::new(icon).is_absolute() && Path::new(icon).exists() {
                    return std::fs::read(icon).ok();
                }
                return icon_png_from_name(icon);
            }
        }
    }
    // Last resort: GTK-based loader (may fail without GTK init)
    systemicons::get_icon(exe_path, 32).ok()
}
