//! Diagnostic helpers for troubleshooting global hotkeys and config (especially on Windows).

pub mod commands;
pub mod config_analyzer;
pub mod hook_probe;
pub mod hotkey_test;
pub mod key_capture;
pub mod report;
pub mod system_check;

/// Low-level keyboard hook probe (Windows).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HookInstallationResult {
    pub success: bool,
    pub hook_handle: String,
    pub error_code: u32,
    pub error_message: String,
    pub thread_id: u32,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KeyCaptureResult {
    pub success: bool,
    pub keys_captured: usize,
    pub duration_secs: u64,
    pub modifier_keys_seen: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HotkeyMatchResult {
    pub success: bool,
    pub hotkey_str: String,
    pub parsed_successfully: bool,
    pub parse_error: Option<String>,
    pub test_events: Vec<TestEvent>,
    pub would_trigger: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TestEvent {
    pub timestamp: String,
    pub event_type: String,
    pub key: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthCheckResult {
    pub dism_status: String,
    pub vc_redist_installed: bool,
    pub registry_checks: Vec<RegistryCheck>,
    pub windows_version: String,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RegistryCheck {
    pub key: String,
    pub exists: bool,
    pub value: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub architecture: String,
    pub kalam_config_path: String,
    pub kalam_config_exists: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub status: String,
    pub duration_ms: u64,
    pub details: String,
    pub errors: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticReport {
    pub timestamp: String,
    pub system_info: SystemInfo,
    pub tests: Vec<TestResult>,
    pub summary: String,
}

/// Optional log lines to embed when saving a markdown report from the UI.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: String,
    pub message: String,
}

pub fn log_diagnostic(message: &str) {
    log::info!("[DIAGNOSTIC] {}", message);
}
