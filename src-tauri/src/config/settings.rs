use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
    pub recording_mode: RecordingMode,
    pub audio_device: Option<String>,
    pub stt_config: STTConfig,
    pub formatting: FormattingConfig,
    pub privacy: PrivacyConfig,
    pub notifications: NotificationConfig,
    pub snippets: Vec<Snippet>,
    pub auto_start: bool,
    pub language: String,
    pub start_in_focus: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Super".to_string(),
            recording_mode: RecordingMode::Hold,
            audio_device: None,
            stt_config: STTConfig::default(),
            formatting: FormattingConfig::default(),
            privacy: PrivacyConfig::default(),
            notifications: NotificationConfig::default(),
            snippets: Vec::new(),
            auto_start: true,
            language: "auto".to_string(),
            start_in_focus: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTConfig {
    pub mode: STTMode,
    pub provider: String,
    pub api_key: Option<String>,
    pub local_model: Option<String>,
    pub vad_preset: VADPreset,
}

impl Default for STTConfig {
    fn default() -> Self {
        Self {
            mode: STTMode::Cloud,
            provider: "groq".to_string(),
            api_key: None,
            local_model: None,
            vad_preset: VADPreset::Balanced,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum STTMode {
    Cloud,
    Local,
    Hybrid,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum VADPreset {
    Fast,
    Balanced,
    Accurate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    pub voice_commands: bool,
    pub filler_word_removal: bool,
    pub auto_punctuation: bool,
    pub custom_rules: Vec<FormattingRule>,
    pub injection_method: InjectionMethod,
    pub keystroke_delay_ms: u64,
    pub clipboard_threshold: usize,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            voice_commands: true,
            filler_word_removal: true,
            auto_punctuation: true,
            custom_rules: Vec::new(),
            injection_method: InjectionMethod::Auto,
            keystroke_delay_ms: 10,
            clipboard_threshold: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionMethod {
    Keystrokes,
    Clipboard,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingRule {
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub history_retention_days: u32,
    pub telemetry_enabled: bool,
    pub sensitive_app_detection: bool,
    pub sensitive_app_patterns: Vec<SensitiveAppPattern>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            history_retention_days: 30,
            telemetry_enabled: false,
            sensitive_app_detection: true,
            sensitive_app_patterns: vec![SensitiveAppPattern {
                pattern: r"(?i)(1password|bitwarden|keepass|lastpass|dashlane)".to_string(),
                pattern_type: PatternType::ProcessName,
                action: PrivacyAction::ForceLocal,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveAppPattern {
    pub pattern: String,
    pub pattern_type: PatternType,
    pub action: PrivacyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    ProcessName,
    WindowTitle,
    BundleId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyAction {
    ForceLocal,
    Block,
    RequireConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub show_completion: bool,
    pub show_errors: bool,
    pub show_updates: bool,
    pub sound_enabled: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            show_completion: false,
            show_errors: true,
            show_updates: true,
            sound_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub trigger: String,
    pub expansion: String,
}
