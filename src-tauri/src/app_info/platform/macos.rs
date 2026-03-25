//! macOS: read `Info.plist` inside `.app` bundles; icons via `systemicons` (NSWorkspace).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::super::capitalize_process_name;

#[derive(Debug, Deserialize)]
struct BundlePlist {
    #[serde(rename = "CFBundleDisplayName")]
    cf_bundle_display_name: Option<String>,
    #[serde(rename = "CFBundleName")]
    cf_bundle_name: Option<String>,
}

fn app_bundle_root(exe_path: &str) -> Option<PathBuf> {
    for ancestor in Path::new(exe_path).ancestors() {
        let name = ancestor.file_name()?.to_str()?;
        if name.ends_with(".app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn read_bundle_display_name(bundle: &Path) -> Option<String> {
    let plist_path = bundle.join("Contents/Info.plist");
    let data = std::fs::read(&plist_path).ok()?;
    let v: BundlePlist = plist::from_bytes(&data).ok()?;
    v.cf_bundle_display_name
        .filter(|s| !s.trim().is_empty())
        .or(v.cf_bundle_name.filter(|s| !s.trim().is_empty()))
}

pub fn resolve_display_name(exe_path: &str) -> Option<String> {
    if let Some(bundle) = app_bundle_root(exe_path) {
        if let Some(n) = read_bundle_display_name(&bundle) {
            return Some(n);
        }
    }
    None
}

pub fn resolve_display_name_from_process_only(_original: &str, normalized: &str) -> Option<String> {
    Some(capitalize_process_name(normalized))
}

pub fn resolve_icon(exe_path: &str) -> Option<Vec<u8>> {
    let path_for_icon = app_bundle_root(exe_path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| exe_path.to_string());
    systemicons::get_icon(&path_for_icon, 32).ok()
}
