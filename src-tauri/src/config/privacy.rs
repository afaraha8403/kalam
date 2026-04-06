//! Sensitive app detection for Hybrid and Auto modes: force Local STT when the foreground app matches patterns.

use super::settings::{AppConfig, PatternType, STTConfig, STTMode};
use regex::Regex;
use std::sync::OnceLock;

/// Lowercase basename of this process's executable (e.g. `kalam.exe`), for ignoring our own overlay/main window in foreground queries.
fn kalam_exe_basename_lower() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| {
                    p.file_name()
                        .map(|s| s.to_string_lossy().to_lowercase())
                })
                .unwrap_or_default()
        })
        .as_str()
}

/// True when `process_name` is this app's executable (overlay or main window counted as "us").
pub fn is_kalam_process(process_name: &str) -> bool {
    let mine = kalam_exe_basename_lower();
    if mine.is_empty() {
        return false;
    }
    let n = process_name.trim().to_lowercase();
    n == mine
}

/// If the live foreground is Kalam, fall back to the last known external app; if live query fails, use cache.
pub fn resolve_external_foreground_app(
    live: Option<(String, String)>,
    cached_external: Option<(String, String)>,
) -> Option<(String, String)> {
    match live {
        Some((p, t)) if !is_kalam_process(&p) => Some((p, t)),
        // Kalam (overlay/main) or empty process id: use last known external app.
        Some(_) => cached_external,
        None => cached_external,
    }
}

#[cfg(windows)]
fn process_name_from_pid(pid: u32) -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle == 0 {
        return None;
    }

    let mut buf = [0u16; 260];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }

    let path = String::from_utf16_lossy(&buf[..size as usize]);
    std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

/// One snapshot from `active_win_pos_rs` (same path as `get_foreground_app`), including fields needed
/// to resolve a Windows injection HWND that matches UWP / ApplicationFrameHost-adjusted PIDs.
#[cfg(windows)]
#[derive(Clone)]
pub struct ActiveWinForegroundDetails {
    pub process_name: String,
    pub title: String,
    pub process_id: u64,
    pub window_id: String,
    pub process_path: std::path::PathBuf,
}

#[cfg(windows)]
pub fn get_active_win_foreground_details() -> Option<ActiveWinForegroundDetails> {
    let window = active_win_pos_rs::get_active_window().ok()?;
    let pid = window.process_id;
    let title = window.title.clone();
    let process_name = process_name_from_pid(pid as u32).unwrap_or_default();
    Some(ActiveWinForegroundDetails {
        process_name,
        title,
        process_id: pid,
        window_id: window.window_id,
        process_path: window.process_path,
    })
}

/// Get the foreground window's process name and title. Returns (process_name, window_title).
pub fn get_foreground_app() -> Option<(String, String)> {
    let window = active_win_pos_rs::get_active_window().ok()?;
    let pid = window.process_id;
    let title = window.title.clone();

    #[cfg(windows)]
    let process_name = process_name_from_pid(pid as u32).unwrap_or_default();

    #[cfg(not(windows))]
    let process_name = {
        use sysinfo::{Pid, ProcessesToUpdate, System};
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All);
        sys.process(Pid::from_u32(pid as u32))
            .map(|p| p.name().to_string_lossy().to_string())
            .unwrap_or_default()
    };

    Some((process_name, title))
}

/// True when Hybrid/Auto, detection is on, and the given app matches a sensitive pattern.
/// Used by the idle loop and `get_context_previews` with resolved external foreground (not live Kalam).
pub fn foreground_matches_sensitive_app_for_process(
    config: &AppConfig,
    process_name: &str,
    window_title: &str,
) -> bool {
    let stt = &config.stt_config;
    if stt.mode != STTMode::Hybrid && stt.mode != STTMode::Auto {
        return false;
    }
    if !config.privacy.sensitive_app_detection || config.privacy.sensitive_app_patterns.is_empty() {
        return false;
    }
    foreground_matches_sensitive_patterns(config, process_name, window_title)
}

fn foreground_matches_sensitive_patterns(
    config: &AppConfig,
    process_name: &str,
    window_title: &str,
) -> bool {
    for pattern in &config.privacy.sensitive_app_patterns {
        let re = match Regex::new(&pattern.pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let matches = match pattern.pattern_type {
            PatternType::ProcessName => re.is_match(process_name),
            PatternType::WindowTitle => re.is_match(window_title),
            PatternType::BundleId => re.is_match(process_name),
        };
        if matches {
            return true;
        }
    }
    false
}

/// If mode is Hybrid or Auto, sensitive app detection is enabled, and patterns exist, check whether the
/// given app matches any pattern. Returns an STTConfig that uses Local mode when a match is found.
pub fn effective_stt_config_for_foreground(
    config: &AppConfig,
    process_name: &str,
    window_title: &str,
) -> STTConfig {
    let stt = &config.stt_config;
    if stt.mode != STTMode::Hybrid && stt.mode != STTMode::Auto {
        return stt.clone();
    }
    if !config.privacy.sensitive_app_detection || config.privacy.sensitive_app_patterns.is_empty() {
        return stt.clone();
    }
    if foreground_matches_sensitive_patterns(config, process_name, window_title) {
        log::info!(
            "Sensitive app detected ({} / {}), forcing Local STT",
            process_name,
            window_title
        );
        let mut local = stt.clone();
        local.mode = STTMode::Local;
        return local;
    }
    stt.clone()
}

/// If mode is Hybrid or Auto, sensitive app detection is enabled, and patterns exist, check whether the
/// foreground app matches any pattern. Returns an STTConfig that uses Local mode when a match is found.
pub fn effective_stt_config(config: &AppConfig) -> STTConfig {
    let (process_name, window_title) = match get_foreground_app() {
        Some(x) => x,
        None => return config.stt_config.clone(),
    };
    effective_stt_config_for_foreground(config, &process_name, &window_title)
}
