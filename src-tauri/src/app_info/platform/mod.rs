//! Platform-specific resolution for display names and icons.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{resolve_display_name, resolve_display_name_from_process_only, resolve_icon};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{resolve_display_name, resolve_display_name_from_process_only, resolve_icon};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{resolve_display_name, resolve_display_name_from_process_only, resolve_icon};

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub fn resolve_display_name(_exe_path: &str) -> Option<String> {
    None
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub fn resolve_display_name_from_process_only(
    _original: &str,
    _normalized: &str,
) -> Option<String> {
    None
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub fn resolve_icon(_exe_path: &str) -> Option<Vec<u8>> {
    None
}
