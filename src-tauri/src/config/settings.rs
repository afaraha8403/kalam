use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Schema version for migrations. Bump when making breaking config changes.
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    #[serde(default = "default_hotkey")]
    pub hotkey: Option<String>,
    #[serde(default)]
    pub toggle_dictation_hotkey: Option<String>,
    #[serde(default, skip_serializing)]
    pub recording_mode: Option<RecordingMode>,
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
    #[serde(default)]
    pub waveform_style: WaveformStyle,
    #[serde(default)]
    pub overlay_position: OverlayPosition,
    #[serde(default)]
    pub overlay_offset_x: i32,
    #[serde(default)]
    pub overlay_offset_y: i32,
    #[serde(default)]
    pub overlay_expand_direction: ExpandDirection,
    /// Master switch: when false, hotkeys and transcription are disabled.
    #[serde(default = "default_dictation_enabled")]
    pub dictation_enabled: bool,
    /// User email collected at onboarding (required to use the app).
    #[serde(default)]
    pub user_email: Option<String>,
    /// Opt-in to marketing communications. Default false.
    #[serde(default)]
    pub marketing_opt_in: bool,
    /// Opt-in to product notifications and updates. Default false (opt-out by default).
    #[serde(default)]
    pub notifications_opt_in: bool,
    /// OS product name when the user completed the email onboarding step (from `get_os_release_info`).
    #[serde(default)]
    pub onboarding_os_name: Option<String>,
    /// OS version string at the same moment (may be empty on some systems).
    #[serde(default)]
    pub onboarding_os_version: Option<String>,
    /// Command mode: dedicated hotkey to create note/task/reminder from voice; optional LLM parsing.
    #[serde(default)]
    pub command_config: CommandConfig,
    /// Update channel: stable (latest release) or beta (pre-releases).
    #[serde(default)]
    pub update_channel: UpdateChannel,
    /// When true, the left sidebar is collapsed to icon-only width; persisted across restarts.
    #[serde(default)]
    pub sidebar_collapsed: bool,
    /// UI theme: fixed light/dark or follow OS appearance (`Auto`).
    #[serde(default)]
    pub theme_preference: ThemePreference,
}

/// Light / Dark fix the shell; Auto follows `prefers-color-scheme` on the frontend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandConfig {
    pub enabled: bool,
    pub hotkey: Option<String>,
    /// "groq" | "openrouter" | "gemini" | "openai" | "anthropic"
    pub provider: Option<String>,
    #[serde(default)]
    pub api_keys: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub models: std::collections::HashMap<String, String>,
    // Legacy fields for backwards compatibility
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing)]
    pub model: Option<String>,
}

fn default_dictation_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum OverlayPosition {
    #[default]
    BottomCenter,
    BottomLeft,
    BottomRight,
    TopCenter,
    TopLeft,
    TopRight,
    CenterLeft,
    CenterRight,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ExpandDirection {
    #[default]
    Up,
    Down,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum WaveformStyle {
    SiriWave,
    #[serde(alias = "PulsingDots")]
    EchoRing,
    #[serde(alias = "Bars")]
    RoundedBars,
    BreathingAura,
    #[serde(alias = "Glitch")]
    Oscilloscope,
    #[serde(alias = "Liquid", alias = "LiquidBlob")]
    NeonPulse,
    /// Legacy waveform names deserialize here (see serde aliases).
    #[default]
    #[serde(alias = "Line", alias = "Symmetric", alias = "Heartbeat", alias = "Snake", alias = "DoubleHelix", alias = "Waves", alias = "CenterSplit")]
    Aurora,
}

fn default_hotkey() -> Option<String> {
    #[cfg(windows)]
    {
        Some("Ctrl+Win".to_string())
    }
    #[cfg(not(windows))]
    {
        Some("Ctrl+Super".to_string())
    }
}

fn default_config_version() -> u32 {
    1
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
            config_version: default_config_version(),
            hotkey: default_hotkey(),
            toggle_dictation_hotkey: None,
            recording_mode: None,
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
            waveform_style: WaveformStyle::default(),
            overlay_position: OverlayPosition::default(),
            overlay_offset_x: 0,
            overlay_offset_y: 0,
            overlay_expand_direction: ExpandDirection::default(),
            dictation_enabled: true,
            user_email: None,
            marketing_opt_in: false,
            notifications_opt_in: false,
            onboarding_os_name: None,
            onboarding_os_version: None,
            command_config: CommandConfig::default(),
            update_channel: UpdateChannel::Stable,
            sidebar_collapsed: false,
            theme_preference: ThemePreference::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingMode {
    Hold,
    Toggle,
}

/// Configuration for dynamic transcription timeout (based on historical latency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionTimeoutConfig {
    #[serde(default = "default_timeout_min_cloud")]
    pub timeout_min_seconds_cloud: u64,
    #[serde(default = "default_timeout_min_local")]
    pub timeout_min_seconds_local: u64,
    #[serde(default = "default_timeout_max_seconds")]
    pub timeout_max_seconds: u64,
    #[serde(default = "default_timeout_multiplier")]
    pub timeout_multiplier: f64,
    #[serde(default = "default_timeout_buffer_seconds")]
    pub timeout_buffer_seconds: u64,
}

fn default_timeout_min_cloud() -> u64 {
    12
}
fn default_timeout_min_local() -> u64 {
    30
}
fn default_timeout_max_seconds() -> u64 {
    120
}
fn default_timeout_multiplier() -> f64 {
    2.0
}
fn default_timeout_buffer_seconds() -> u64 {
    10
}

impl Default for TranscriptionTimeoutConfig {
    fn default() -> Self {
        Self {
            timeout_min_seconds_cloud: default_timeout_min_cloud(),
            timeout_min_seconds_local: default_timeout_min_local(),
            timeout_max_seconds: default_timeout_max_seconds(),
            timeout_multiplier: default_timeout_multiplier(),
            timeout_buffer_seconds: default_timeout_buffer_seconds(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct STTConfig {
    pub mode: STTMode,
    pub provider: String,
    #[serde(default)]
    pub api_keys: std::collections::HashMap<String, String>,
    // Legacy field for backwards compatibility; migrated into api_keys on load.
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    pub local_model: Option<String>,
    pub vad_preset: VADPreset,
    #[serde(default)]
    pub audio_filter: crate::audio::filter::AudioFilterConfig,
    #[serde(default)]
    pub transcription_timeout: TranscriptionTimeoutConfig,
}

impl Default for STTConfig {
    fn default() -> Self {
        Self {
            mode: STTMode::Cloud,
            provider: "groq".to_string(),
            api_keys: std::collections::HashMap::new(),
            api_key: None,
            local_model: None,
            vad_preset: VADPreset::Balanced,
            audio_filter: crate::audio::filter::AudioFilterConfig::default(),
            transcription_timeout: TranscriptionTimeoutConfig::default(),
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
    /// Process names (lowercase, e.g. "notepad.exe") that should always use
    /// Clipboard injection regardless of injection_method / clipboard_threshold.
    /// Win11 Notepad and similar TSF-heavy apps corrupt rapid keystroke bursts.
    #[serde(default = "default_force_clipboard_apps")]
    pub force_clipboard_apps: Vec<String>,
}

fn default_retry_attempts() -> u32 {
    3
}
fn default_retry_delay_ms() -> u64 {
    100
}
fn default_force_clipboard_apps() -> Vec<String> {
    vec!["notepad.exe".to_string()]
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
            force_clipboard_apps: default_force_clipboard_apps(),
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
                pattern: r"(?i)(1password|bitwarden|keepass|lastpass|dashlane|nordpass)".to_string(),
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

/// Only `ForceLocal` is implemented. Legacy `Block` / `RequireConfirmation` in JSON map to `ForceLocal` on load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivacyAction {
    #[default]
    ForceLocal,
}

impl Serialize for PrivacyAction {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("ForceLocal")
    }
}

impl<'de> Deserialize<'de> for PrivacyAction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "ForceLocal" | "Block" | "RequireConfirmation" => Self::ForceLocal,
            _ => Self::ForceLocal,
        })
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "PascalCase")]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
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
