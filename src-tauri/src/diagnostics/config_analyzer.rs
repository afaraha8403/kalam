use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConfigAnalysis {
    pub config_path: String,
    pub config_exists: bool,
    pub hotkey: Option<String>,
    pub toggle_hotkey: Option<String>,
    pub dictation_enabled: bool,
    pub recording_mode: String,
    pub parsing_errors: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Analyze Kalam `config.json` using the same directory as `ConfigManager` (no writes).
pub fn analyze_config() -> Result<ConfigAnalysis, String> {
    let kalam_dir = crate::config::get_kalam_dir().map_err(|e| e.to_string())?;
    let config_path_buf: PathBuf = kalam_dir.join("config.json");
    let config_path = config_path_buf.to_string_lossy().to_string();
    let config_exists = config_path_buf.exists();

    log::info!("[DIAGNOSTIC] Config path: {config_path}, exists: {config_exists}");

    if !config_exists {
        return Ok(ConfigAnalysis {
            config_path: config_path.clone(),
            config_exists: false,
            hotkey: None,
            toggle_hotkey: None,
            dictation_enabled: false,
            recording_mode: "Unknown".to_string(),
            parsing_errors: vec!["Config file not found".to_string()],
            warnings: vec![],
            recommendations: vec![
                "Run Kalam at least once to create a default config.".to_string(),
                format!("Expected file at: {config_path}"),
            ],
        });
    }

    let config_content = match std::fs::read_to_string(&config_path_buf) {
        Ok(content) => content,
        Err(e) => {
            return Ok(ConfigAnalysis {
                config_path,
                config_exists: true,
                hotkey: None,
                toggle_hotkey: None,
                dictation_enabled: false,
                recording_mode: "Unknown".to_string(),
                parsing_errors: vec![format!("Failed to read config: {e}")],
                warnings: vec![],
                recommendations: vec!["Check file permissions.".to_string()],
            });
        }
    };

    let config: Value = match serde_json::from_str(&config_content) {
        Ok(val) => val,
        Err(e) => {
            return Ok(ConfigAnalysis {
                config_path,
                config_exists: true,
                hotkey: None,
                toggle_hotkey: None,
                dictation_enabled: false,
                recording_mode: "Unknown".to_string(),
                parsing_errors: vec![format!("Failed to parse JSON: {e}")],
                warnings: vec![],
                recommendations: vec![
                    "Config may be corrupted. Back up and remove config.json, then restart Kalam."
                        .to_string(),
                ],
            });
        }
    };

    let hotkey = config
        .get("hotkey")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let toggle_hotkey = config
        .get("toggle_dictation_hotkey")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let dictation_enabled = config
        .get("dictation_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let recording_mode = config
        .get("recording_mode")
        .map(|v| {
            if v.is_null() {
                "Both".to_string()
            } else {
                v.as_str().unwrap_or("Unknown").to_string()
            }
        })
        .unwrap_or_else(|| "Both".to_string());

    log::info!(
        "[DIAGNOSTIC] hotkey={hotkey:?} toggle={toggle_hotkey:?} dictation_enabled={dictation_enabled} recording_mode={recording_mode}"
    );

    let mut warnings = vec![];
    let mut recommendations = vec![];

    if !dictation_enabled {
        warnings.push("Dictation is disabled in config — hotkeys will not start recording.".to_string());
        recommendations
            .push("Enable dictation in Settings or set dictation_enabled to true.".to_string());
    }

    if hotkey.is_none() && toggle_hotkey.is_none() {
        warnings.push("No hold or toggle hotkey is configured.".to_string());
        recommendations.push("Configure at least one dictation hotkey in Settings.".to_string());
    }

    if let Some(ref hk) = hotkey {
        let lower = hk.to_lowercase();
        if lower.contains("win") || lower.contains("meta") {
            recommendations.push(
                "This hotkey uses the Windows key — ensure Group Policy / registry is not blocking it (e.g. NoWinKeys)."
                    .to_string(),
            );
        }
        let parts: Vec<&str> = hk.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            warnings.push(format!(
                "Hold hotkey \"{hk}\" has fewer than two parts — use modifier+key (e.g. Ctrl+Win)."
            ));
        }
    }

    if let Some(ref hk) = toggle_hotkey {
        let parts: Vec<&str> = hk.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            warnings.push(format!(
                "Toggle hotkey \"{hk}\" has fewer than two parts — use modifier+key."
            ));
        }
    }

    match recording_mode.as_str() {
        "Hold" => {
            if hotkey.is_none() {
                warnings.push("Recording mode is Hold but no hold hotkey is set.".to_string());
            }
        }
        "Toggle" => {
            if toggle_hotkey.is_none() {
                warnings.push("Recording mode is Toggle but no toggle hotkey is set.".to_string());
            }
        }
        "Both" => {
            if hotkey.is_none() {
                warnings.push("Recording mode is Both (default) but no hold hotkey is set.".to_string());
            }
            if toggle_hotkey.is_none() {
                warnings.push("Recording mode is Both (default) but no toggle hotkey is set.".to_string());
            }
        }
        _ => {
            if recording_mode != "Unknown" {
                warnings.push(format!("Unexpected recording_mode value: {recording_mode}"));
            }
        }
    }

    Ok(ConfigAnalysis {
        config_path,
        config_exists: true,
        hotkey,
        toggle_hotkey,
        dictation_enabled,
        recording_mode,
        parsing_errors: vec![],
        warnings,
        recommendations,
    })
}
