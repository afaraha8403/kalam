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
    #[serde(default)]
    pub logging: LoggingConfig,
    pub snippets: Vec<Snippet>,
    pub auto_start: bool,
    /// Ordered list of recognition languages. First is the default/active; toggle hotkey swaps first and second.
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    /// Legacy: migrated into languages on load. Not serialized.
    #[serde(default, skip_serializing)]
    pub language: Option<String>,
    /// Legacy: migrated into languages on load. Not serialized.
    #[serde(default, skip_serializing)]
    pub secondary_language: Option<String>,
    #[serde(default)]
    pub language_toggle_hotkey: Option<String>,
    pub start_in_focus: bool,
    #[serde(default = "default_min_hold_ms")]
    pub min_hold_ms: u64,
    #[serde(default)]
    pub onboarding_complete: bool,
}

fn default_hotkey() -> String {
    #[cfg(windows)]
    {
        "Ctrl+Win".to_string()
    }
    #[cfg(not(windows))]
    {
        "Ctrl+Super".to_string()
    }
}

fn default_min_hold_ms() -> u64 {
    300
}

fn default_languages() -> Vec<String> {
    vec!["en".to_string()]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            recording_mode: RecordingMode::Hold,
            audio_device: None,
            stt_config: STTConfig::default(),
            formatting: FormattingConfig::default(),
            privacy: PrivacyConfig::default(),
            notifications: NotificationConfig::default(),
            logging: LoggingConfig::default(),
            snippets: Vec::new(),
            auto_start: true,
            languages: default_languages(),
            language: None,
            secondary_language: None,
            language_toggle_hotkey: None,
            start_in_focus: true,
            min_hold_ms: default_min_hold_ms(),
            onboarding_complete: false,
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

impl STTConfig {
    /// Map VAD preset (Fast, Balanced, Accurate) to VADConfig for the audio pipeline.
    pub fn vad_config(&self) -> crate::audio::vad::VADConfig {
        match self.vad_preset {
            VADPreset::Fast => crate::audio::vad::VADConfig::fast(),
            VADPreset::Balanced => crate::audio::vad::VADConfig::default(),
            VADPreset::Accurate => crate::audio::vad::VADConfig::accurate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

fn default_retry_attempts() -> u32 {
    3
}
fn default_retry_delay_ms() -> u64 {
    100
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
            retry_attempts: 3,
            retry_delay_ms: 100,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrivacyAction {
    ForceLocal,
    Block,
    RequireConfirmation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: LogLevel,
    pub max_records: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: LogLevel::Info,
            max_records: 2000,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl LogLevel {
    /// Returns the log::Level filter for this level (Off = nothing passes).
    pub fn to_log_filter(self) -> Option<log::Level> {
        use log::Level;
        match self {
            LogLevel::Off => None,
            LogLevel::Error => Some(Level::Error),
            LogLevel::Warn => Some(Level::Warn),
            LogLevel::Info => Some(Level::Info),
            LogLevel::Debug => Some(Level::Debug),
        }
    }
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
