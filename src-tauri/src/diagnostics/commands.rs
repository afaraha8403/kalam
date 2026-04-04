//! Tauri commands for diagnostics (Settings → Advanced → Diagnostics).

use crate::diagnostics;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::time::Instant;

static DIAG_TEST_RESULTS: Lazy<Mutex<Vec<diagnostics::TestResult>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

const MAX_ACCUMULATED_RESULTS: usize = 64;

fn push_diagnostic_test_result(r: diagnostics::TestResult) {
    if let Ok(mut g) = DIAG_TEST_RESULTS.lock() {
        g.push(r);
        while g.len() > MAX_ACCUMULATED_RESULTS {
            g.remove(0);
        }
    }
}

fn drain_diagnostic_test_results() -> Vec<diagnostics::TestResult> {
    DIAG_TEST_RESULTS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default()
}

#[tauri::command]
pub async fn run_hook_installation_test() -> Result<diagnostics::HookInstallationResult, String> {
    let prev = crate::app_log::escalate();
    let start = Instant::now();
    let out = diagnostics::hotkey_test::test_hook_installation();
    let elapsed = start.elapsed().as_millis() as u64;
    crate::app_log::deescalate(prev);
    match &out {
        Ok(h) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Hook install probe".to_string(),
            status: if h.success {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            duration_ms: elapsed,
            details: format!(
                "hook_handle={} error_code={} thread_id={}",
                h.hook_handle, h.error_code, h.thread_id
            ),
            errors: if h.success {
                vec![]
            } else {
                vec![h.error_message.clone()]
            },
            recommendations: h.recommendations.clone(),
        }),
        Err(e) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Hook install probe".to_string(),
            status: "error".to_string(),
            duration_ms: elapsed,
            details: e.clone(),
            errors: vec![e.clone()],
            recommendations: vec![],
        }),
    }
    out
}

#[tauri::command]
pub async fn run_key_capture_test(
    duration_secs: u64,
) -> Result<diagnostics::KeyCaptureResult, String> {
    let prev = crate::app_log::escalate();
    let start = Instant::now();
    let out = diagnostics::hotkey_test::test_key_capture(duration_secs);
    let elapsed = start.elapsed().as_millis() as u64;
    crate::app_log::deescalate(prev);
    match &out {
        Ok(k) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Key capture".to_string(),
            status: if k.success {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            duration_ms: elapsed,
            details: format!(
                "keys_captured={} duration_secs={}",
                k.keys_captured, k.duration_secs
            ),
            errors: k.errors.clone(),
            recommendations: vec![],
        }),
        Err(e) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Key capture".to_string(),
            status: "error".to_string(),
            duration_ms: elapsed,
            details: e.clone(),
            errors: vec![e.clone()],
            recommendations: vec![],
        }),
    }
    out
}

#[tauri::command]
pub async fn run_hotkey_matching_test(
    hotkey_str: String,
) -> Result<diagnostics::HotkeyMatchResult, String> {
    let prev = crate::app_log::escalate();
    let start = Instant::now();
    let out = diagnostics::hotkey_test::test_hotkey_matching(&hotkey_str);
    let elapsed = start.elapsed().as_millis() as u64;
    crate::app_log::deescalate(prev);
    match &out {
        Ok(m) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Hotkey match (10s)".to_string(),
            status: if m.success {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            duration_ms: elapsed,
            details: format!(
                "hotkey={} parsed={} would_trigger={}",
                m.hotkey_str, m.parsed_successfully, m.would_trigger
            ),
            errors: m.errors.clone(),
            recommendations: vec![],
        }),
        Err(e) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Hotkey match (10s)".to_string(),
            status: "error".to_string(),
            duration_ms: elapsed,
            details: e.clone(),
            errors: vec![e.clone()],
            recommendations: vec![],
        }),
    }
    out
}

#[tauri::command]
pub async fn analyze_kalam_config_diagnostic(
) -> Result<diagnostics::config_analyzer::ConfigAnalysis, String> {
    let prev = crate::app_log::escalate();
    let start = Instant::now();
    let out = diagnostics::config_analyzer::analyze_config();
    let elapsed = start.elapsed().as_millis() as u64;
    crate::app_log::deescalate(prev);
    match &out {
        Ok(a) => {
            let errors = a.parsing_errors.clone();
            let status = if errors.is_empty() && a.warnings.is_empty() {
                "pass".to_string()
            } else if errors.is_empty() {
                "warn".to_string()
            } else {
                "fail".to_string()
            };
            push_diagnostic_test_result(diagnostics::TestResult {
                test_name: "Analyze config".to_string(),
                status,
                duration_ms: elapsed,
                details: format!(
                    "path={} exists={} dictation_enabled={}",
                    a.config_path, a.config_exists, a.dictation_enabled
                ),
                errors,
                recommendations: a.recommendations.clone(),
            });
        }
        Err(e) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "Analyze config".to_string(),
            status: "error".to_string(),
            duration_ms: elapsed,
            details: e.clone(),
            errors: vec![e.clone()],
            recommendations: vec![],
        }),
    }
    out
}

#[tauri::command]
pub async fn run_system_health_check() -> Result<diagnostics::HealthCheckResult, String> {
    let prev = crate::app_log::escalate();
    let start = Instant::now();
    let out = diagnostics::system_check::run_health_check();
    let elapsed = start.elapsed().as_millis() as u64;
    crate::app_log::deescalate(prev);
    match &out {
        Ok(h) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "System health (DISM)".to_string(),
            status: "info".to_string(),
            duration_ms: elapsed,
            details: format!(
                "dism_status={} vc_redist={}",
                h.dism_status, h.vc_redist_installed
            ),
            errors: vec![],
            recommendations: h.recommendations.clone(),
        }),
        Err(e) => push_diagnostic_test_result(diagnostics::TestResult {
            test_name: "System health (DISM)".to_string(),
            status: "error".to_string(),
            duration_ms: elapsed,
            details: e.clone(),
            errors: vec![e.clone()],
            recommendations: vec![],
        }),
    }
    out
}

#[tauri::command]
pub fn get_modifier_state() -> (bool, bool, bool, bool) {
    let start = Instant::now();
    let out = crate::hotkey::get_modifier_state();
    let elapsed = start.elapsed().as_millis() as u64;
    push_diagnostic_test_result(diagnostics::TestResult {
        test_name: "Modifier state snapshot".to_string(),
        status: "info".to_string(),
        duration_ms: elapsed,
        details: format!(
            "ctrl={} alt={} shift={} meta={}",
            out.0, out.1, out.2, out.3
        ),
        errors: vec![],
        recommendations: vec![],
    });
    out
}

#[tauri::command]
pub fn get_diagnostic_system_info() -> diagnostics::SystemInfo {
    diagnostics::system_check::gather_system_info()
}

#[tauri::command]
pub async fn generate_diagnostics_report() -> Result<diagnostics::DiagnosticReport, String> {
    Ok(diagnostics::report::build_report(
        vec![],
        "Run tests under Settings → Advanced → Diagnostics, then save a report to attach structured rows and a log excerpt."
            .to_string(),
    ))
}

#[tauri::command]
pub async fn save_diagnostics_report_to_file() -> Result<String, String> {
    let tests = drain_diagnostic_test_results();
    let prev = crate::app_log::escalate();
    log::info!(
        "Saving diagnostic report ({} structured test row(s))",
        tests.len()
    );
    let report = diagnostics::report::build_report(
        tests,
        "Saved from Kalam Settings → Advanced → Diagnostics. Includes structured test rows from this session (since last save) and a log excerpt."
            .to_string(),
    );
    let logs = crate::app_log::recent_log_entries(200);
    crate::app_log::deescalate(prev);
    diagnostics::report::save_report_markdown(&report, &logs)
}
