//! Sensitive app detection for Hybrid mode: force Local STT when foreground app matches patterns.

use super::settings::{AppConfig, PatternType, PrivacyAction, STTConfig, STTMode};
use regex::Regex;
use sysinfo::System;

/// Get the foreground window's process name and title. Returns (process_name, window_title).
pub fn get_foreground_app() -> Option<(String, String)> {
    let window = active_win_pos_rs::get_active_window().ok()?;
    let pid = window.process_id;
    let title = window.title.clone();
    let mut sys = System::new_all();
    sys.refresh_all();
    let pid_sys = sysinfo::Pid::from_u32(pid as u32);
    let process_name = sys
        .process(pid_sys)
        .map(|p| p.name().to_string_lossy().to_string())
        .unwrap_or_default();
    Some((process_name, title))
}

/// If Hybrid mode is on and sensitive app detection is enabled, check whether the
/// foreground app matches any ForceLocal pattern. Returns an STTConfig that uses
/// Local mode when a match is found.
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
    for pattern in &config.privacy.sensitive_app_patterns {
        if pattern.action != PrivacyAction::ForceLocal {
            continue;
        }
        let re = match Regex::new(&pattern.pattern) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let matches = match pattern.pattern_type {
            PatternType::ProcessName => re.is_match(&process_name),
            PatternType::WindowTitle => re.is_match(&window_title),
            PatternType::BundleId => re.is_match(&process_name),
        };
        if matches {
            log::info!(
                "Sensitive app detected ({} / {}), forcing Local STT",
                process_name,
                window_title
            );
            let mut local = stt.clone();
            local.mode = STTMode::Local;
            return local;
        }
    }
    stt.clone()
}
