//! Tauri commands for the in-app Diagnostics page.

use crate::diagnostics;

#[tauri::command]
pub async fn run_hook_installation_test() -> Result<diagnostics::HookInstallationResult, String> {
    diagnostics::hotkey_test::test_hook_installation()
}

#[tauri::command]
pub async fn run_key_capture_test(duration_secs: u64) -> Result<diagnostics::KeyCaptureResult, String> {
    diagnostics::hotkey_test::test_key_capture(duration_secs)
}

#[tauri::command]
pub async fn run_hotkey_matching_test(
    hotkey_str: String,
) -> Result<diagnostics::HotkeyMatchResult, String> {
    diagnostics::hotkey_test::test_hotkey_matching(&hotkey_str)
}

#[tauri::command]
pub async fn analyze_kalam_config_diagnostic(
) -> Result<diagnostics::config_analyzer::ConfigAnalysis, String> {
    diagnostics::config_analyzer::analyze_config()
}

#[tauri::command]
pub async fn run_system_health_check() -> Result<diagnostics::HealthCheckResult, String> {
    diagnostics::system_check::run_health_check()
}

#[tauri::command]
pub fn get_modifier_state() -> (bool, bool, bool, bool) {
    crate::hotkey::get_modifier_state()
}

#[tauri::command]
pub fn get_diagnostic_system_info() -> diagnostics::SystemInfo {
    diagnostics::system_check::gather_system_info()
}

#[tauri::command]
pub async fn generate_diagnostics_report() -> Result<diagnostics::DiagnosticReport, String> {
    Ok(diagnostics::report::build_report(
        vec![],
        "Run tests on the Diagnostics page, then save a report to attach full results.".to_string(),
    ))
}

#[tauri::command]
pub async fn save_diagnostics_report_to_file() -> Result<String, String> {
    let report = diagnostics::report::build_report(
        vec![],
        "Saved from Kalam Diagnostics (run tests in the app for detailed rows).".to_string(),
    );
    diagnostics::report::save_report_markdown(&report, &[])
}
