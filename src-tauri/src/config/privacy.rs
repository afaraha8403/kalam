//! Sensitive app detection for Hybrid and Auto modes: force Local STT when the foreground app matches patterns.

use super::settings::{AppConfig, PatternType, STTConfig, STTMode};
use regex::Regex;

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

/// True when Hybrid/Auto, detection is on, and the current foreground matches a sensitive pattern.
/// Used for overlay “peek” when focus enters a sensitive app (no dictation yet).
pub fn foreground_matches_sensitive_app(config: &AppConfig) -> bool {
    let stt = &config.stt_config;
    if stt.mode != STTMode::Hybrid && stt.mode != STTMode::Auto {
        return false;
    }
    if !config.privacy.sensitive_app_detection || config.privacy.sensitive_app_patterns.is_empty() {
        return false;
    }
    let (process_name, window_title) = match get_foreground_app() {
        Some(x) => x,
        None => return false,
    };
    foreground_matches_sensitive_patterns(config, &process_name, &window_title)
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
/// foreground app matches any pattern. Returns an STTConfig that uses Local mode when a match is found.
pub fn effective_stt_config(config: &AppConfig) -> STTConfig {
    let stt = &config.stt_config;
    if stt.mode != STTMode::Hybrid && stt.mode != STTMode::Auto {
        return stt.clone();
    }
    if !config.privacy.sensitive_app_detection || config.privacy.sensitive_app_patterns.is_empty() {
        return stt.clone();
    }
    let (process_name, window_title) = match get_foreground_app() {
        Some(x) => x,
        None => return stt.clone(),
    };
    if foreground_matches_sensitive_patterns(config, &process_name, &window_title) {
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
