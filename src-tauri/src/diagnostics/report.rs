use crate::diagnostics::{DiagnosticReport, LogEntry, TestResult};
use std::fs;
use std::path::PathBuf;

/// Build a report snapshot (tests array is filled by the caller or left empty).
pub fn build_report(tests: Vec<TestResult>, summary: String) -> DiagnosticReport {
    DiagnosticReport {
        timestamp: chrono::Local::now().to_rfc3339(),
        system_info: crate::diagnostics::system_check::gather_system_info(),
        tests,
        summary,
    }
}

/// Save markdown report under `.kalam/diagnostics/` (same root as `config.json`).
pub fn save_report_markdown(report: &DiagnosticReport, logs: &[LogEntry]) -> Result<String, String> {
    let base = crate::config::get_kalam_dir()
        .map_err(|e| e.to_string())?
        .join("diagnostics");
    fs::create_dir_all(&base).map_err(|e| format!("Failed to create diagnostics dir: {e}"))?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H%M%S");
    let filename = format!("kalam-diagnostics-{timestamp}.md");
    let filepath: PathBuf = base.join(&filename);

    let md = format_report_markdown(report, logs);
    fs::write(&filepath, &md).map_err(|e| format!("Failed to write report: {e}"))?;

    Ok(filepath.to_string_lossy().to_string())
}

fn format_report_markdown(report: &DiagnosticReport, logs: &[LogEntry]) -> String {
    let mut md = String::new();
    md.push_str("# Kalam diagnostic report\n\n");
    md.push_str(&format!("**Generated:** {}\n\n", report.timestamp));

    md.push_str("## System\n\n");
    md.push_str(&format!("- **OS:** {}\n", report.system_info.os_name));
    md.push_str(&format!("- **Version:** {}\n", report.system_info.os_version));
    md.push_str(&format!("- **Arch:** {}\n", report.system_info.architecture));
    md.push_str(&format!(
        "- **Config:** {}\n",
        report.system_info.kalam_config_path
    ));
    md.push_str(&format!(
        "- **Config exists:** {}\n\n",
        report.system_info.kalam_config_exists
    ));

    md.push_str("## Tests\n\n");
    if report.tests.is_empty() {
        md.push_str("_No structured test rows were attached._\n\n");
    } else {
        for t in &report.tests {
            md.push_str(&format!("### {}\n", t.test_name));
            md.push_str(&format!("- **Status:** {}\n", t.status));
            md.push_str(&format!("- **Duration:** {} ms\n", t.duration_ms));
            md.push_str(&format!("- **Details:** {}\n", t.details));
            if !t.errors.is_empty() {
                md.push_str("- **Errors:**\n");
                for e in &t.errors {
                    md.push_str(&format!("  - {e}\n"));
                }
            }
            if !t.recommendations.is_empty() {
                md.push_str("- **Recommendations:**\n");
                for r in &t.recommendations {
                    md.push_str(&format!("  - {r}\n"));
                }
            }
            md.push('\n');
        }
    }

    if !logs.is_empty() {
        md.push_str("## Log excerpt\n\n```\n");
        for entry in logs {
            let ts = entry.timestamp.format("%H:%M:%S");
            md.push_str(&format!(
                "[{ts}] [{}] {}\n",
                entry.level, entry.message
            ));
        }
        md.push_str("```\n\n");
    }

    md.push_str("## Summary\n\n");
    md.push_str(&report.summary);
    md.push('\n');
    md
}
