use crate::diagnostics::{log_diagnostic, HealthCheckResult, SystemInfo};
use std::process::Command;

/// Gather system information for the diagnostics UI.
pub fn gather_system_info() -> SystemInfo {
    let os_name = std::env::consts::OS.to_string();
    let os_version = get_windows_version();
    let architecture = std::env::consts::ARCH.to_string();

    let (kalam_config_path, kalam_config_exists) =
        match crate::config::get_kalam_dir() {
            Ok(dir) => {
                let p = dir.join("config.json");
                let exists = p.exists();
                (p.to_string_lossy().to_string(), exists)
            }
            Err(_) => (String::new(), false),
        };

    SystemInfo {
        os_name,
        os_version,
        architecture,
        kalam_config_path,
        kalam_config_exists,
    }
}

/// Run system health checks (DISM snapshot; registry checks are optional / manual).
pub fn run_health_check() -> Result<HealthCheckResult, String> {
    log_diagnostic("=== System Health Check ===");

    let windows_version = get_windows_version();
    log_diagnostic(&format!("OS version string: {windows_version}"));

    log_diagnostic("Checking DISM component store health...");
    let dism_status = check_dism_health();
    log_diagnostic(&format!("DISM status: {dism_status}"));

    let registry_checks: Vec<crate::diagnostics::RegistryCheck> = vec![];

    let mut recommendations = vec![];
    if dism_status.contains("repairable") {
        recommendations.push(
            "Windows component store may need repair. Run: DISM /Online /Cleanup-Image /RestoreHealth"
                .to_string(),
        );
    }

    recommendations.push(
        "Optional: in Registry Editor check NoWinKeys and LowLevelHooksTimeout if Win-key or hooks misbehave."
            .to_string(),
    );

    Ok(HealthCheckResult {
        dism_status,
        vc_redist_installed: true,
        registry_checks,
        windows_version,
        recommendations,
    })
}

fn get_windows_version() -> String {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::GetVersionExW;
        use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

        unsafe {
            let mut osvi: OSVERSIONINFOW = std::mem::zeroed();
            osvi.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;

            if GetVersionExW(&mut osvi) != 0 {
                format!(
                    "{}.{} (Build {})",
                    osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber
                )
            } else {
                "Unknown".to_string()
            }
        }
    }
    #[cfg(not(windows))]
    {
        format!(
            "{} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    }
}

fn check_dism_health() -> String {
    #[cfg(windows)]
    {
        match Command::new("DISM")
            .args(["/Online", "/Cleanup-Image", "/CheckHealth"])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if stdout.contains("repairable") || stderr.contains("repairable") {
                    "Component store is repairable".to_string()
                } else if stdout.contains("healthy") || stderr.contains("healthy") {
                    "Component store is healthy".to_string()
                } else {
                    "Unknown status (may require elevation)".to_string()
                }
            }
            Err(_) => "Failed to run DISM (may need admin)".to_string(),
        }
    }
    #[cfg(not(windows))]
    {
        "N/A (not Windows)".to_string()
    }
}
