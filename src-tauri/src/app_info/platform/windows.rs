//! Windows: `VersionInfo` for friendly names, `systemicons` for PNG icons.

use super::super::capitalize_process_name;

pub fn resolve_display_name(exe_path: &str) -> Option<String> {
    let info = win32_version_info::VersionInfo::from_file(exe_path).ok()?;
    let fd = info.file_description.trim();
    if !fd.is_empty() && fd != " " {
        return Some(trim_product_suffix(fd));
    }
    let pn = info.product_name.trim();
    if !pn.is_empty() {
        return Some(trim_product_suffix(pn));
    }
    None
}

fn trim_product_suffix(s: &str) -> String {
    s.split(" - ")
        .next()
        .unwrap_or(s)
        .trim()
        .to_string()
}

pub fn resolve_display_name_from_process_only(_original: &str, normalized: &str) -> Option<String> {
    // Without a full path, version resources are not reliably readable.
    Some(capitalize_process_name(normalized))
}

pub fn resolve_icon(exe_path: &str) -> Option<Vec<u8>> {
    systemicons::get_icon(exe_path, 32).ok()
}
