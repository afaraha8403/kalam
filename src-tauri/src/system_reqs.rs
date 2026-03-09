use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
pub struct HardwareCheckResult {
    pub can_run: bool,
    pub reason: Option<String>,
}

pub fn check_model_requirements(model_id: &str) -> HardwareCheckResult {
    let mut sys = System::new_all();
    sys.refresh_memory();

    // Total memory is in bytes
    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes as f64 / 1_073_741_824.0;

    match model_id {
        "sensevoice" => {
            // SenseVoice needs > 2GB RAM
            if total_memory_gb < 2.0 {
                HardwareCheckResult {
                    can_run: false,
                    reason: Some(format!(
                        "Requires at least 2GB RAM (You have {:.1}GB)",
                        total_memory_gb
                    )),
                }
            } else {
                HardwareCheckResult {
                    can_run: true,
                    reason: None,
                }
            }
        }
        "whisper_base" => {
            // Whisper Base needs > 4GB RAM
            if total_memory_gb < 4.0 {
                HardwareCheckResult {
                    can_run: false,
                    reason: Some(format!(
                        "Requires at least 4GB RAM (You have {:.1}GB)",
                        total_memory_gb
                    )),
                }
            } else {
                HardwareCheckResult {
                    can_run: true,
                    reason: None,
                }
            }
        }
        _ => HardwareCheckResult {
            can_run: true,
            reason: None,
        },
    }
}
