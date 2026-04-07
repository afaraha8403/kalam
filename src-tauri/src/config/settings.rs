use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Dictation modes (Phase 1): recipes + per-mode STT/LLM refs ---

/// Per-mode STT or LLM provider + model. Empty provider means "use global default"
/// (`stt_config` for voice, `default_llm_provider`/`default_llm_model` for LLM).
/// Legacy `"inherit"` values are treated the same as empty during resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ModeModelRef {
    pub provider: String,
    pub model_id: String,
}

impl ModeModelRef {
    /// True when this ref defers to the global default (empty or legacy "inherit").
    pub fn is_default(&self) -> bool {
        let p = self.provider.trim();
        p.is_empty() || p.eq_ignore_ascii_case("inherit")
    }
}

/// Context awareness toggles per mode (Phase 4 will read the screen; data model only in Phase 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ModeContextConfig {
    pub enabled: bool,
    pub read_app: bool,
    pub read_clipboard: bool,
    pub read_selection: bool,
    pub include_system_info: bool,
}

/// Default accent when `accent_color` is empty (CSS color, usually OKLCH).
pub fn default_accent_for_mode_id(id: &str) -> String {
    match id {
        "default" | "voice" => "oklch(68% 0.1 240)".to_string(),
        "email" => "oklch(68% 0.1 220)".to_string(),
        "message" => "oklch(70% 0.1 160)".to_string(),
        "notes" => "oklch(72% 0.06 85)".to_string(),
        _ => {
            let mut hash: u32 = 0;
            for b in id.bytes() {
                hash = b as u32 + hash.wrapping_mul(31);
            }
            let hue = (hash % 360) as i32;
            format!("oklch(68% 0.1 {hue})")
        }
    }
}

/// Resolved accent for UI (stored value or id-based default).
pub fn effective_accent_color(mode: &DictationMode) -> String {
    let t = mode.accent_color.trim();
    if t.is_empty() {
        default_accent_for_mode_id(&mode.id)
    } else {
        mode.accent_color.clone()
    }
}

/// A dictation "recipe": name, models, AI instructions, polish default, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictationMode {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    /// CSS color for overlay + main UI (OKLCH, hex, etc.). Empty = derive from `id`.
    #[serde(default)]
    pub accent_color: String,
    #[serde(default)]
    pub ai_instructions: String,
    #[serde(default)]
    pub voice_model: ModeModelRef,
    #[serde(default)]
    pub language_model: ModeModelRef,
    /// When true, polish instructions are included in the LLM call.
    #[serde(default)]
    pub polish: bool,
    #[serde(default)]
    pub context: ModeContextConfig,
    #[serde(default)]
    pub auto_activate_rules: Vec<AutoActivateRule>,
    /// Shipped templates (Email, Notes, …); user may still delete most built-ins except Voice.
    #[serde(default)]
    pub is_builtin: bool,
    #[serde(default = "default_true")]
    pub is_deletable: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

/// Advanced: OpenAI-compatible chat/completions endpoint (custom base URL + key + model).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CustomOpenAiEndpoint {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
}

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
    /// Compact pill vs full panel when recording starts (Phase 6).
    #[serde(default)]
    pub overlay_active_preference: OverlayActivePreference,
    /// When true, idle overlay pill stays fully opaque (Phase 6).
    #[serde(default)]
    pub overlay_always_visible: bool,
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
    /// Unified API keys by provider id (`groq`, `openai`, …). Phase 2 migration merges legacy `stt_config.api_keys` / `command_config.api_keys`.
    #[serde(default)]
    pub provider_keys: HashMap<String, String>,
    /// Global fallback LLM provider id (e.g. "groq", "openai"). Used when a mode has no LLM configured.
    #[serde(default)]
    pub default_llm_provider: Option<String>,
    /// Global fallback LLM model id (e.g. "llama-3.3-70b-versatile"). Paired with `default_llm_provider`.
    #[serde(default)]
    pub default_llm_model: Option<String>,
    /// Optional custom OpenAI-compatible endpoint for LLM (provider = `custom_openai`).
    #[serde(default)]
    pub custom_openai_endpoint: Option<CustomOpenAiEndpoint>,
    /// Update channel: stable (latest release) or beta (pre-releases).
    #[serde(default)]
    pub update_channel: UpdateChannel,
    /// When true, do not auto-switch update channel to Beta on pre-release app builds (user set channel in About).
    #[serde(default)]
    pub update_channel_locked: bool,
    /// When true, the left sidebar is collapsed to icon-only width; persisted across restarts.
    #[serde(default)]
    pub sidebar_collapsed: bool,
    /// UI theme: fixed light/dark or follow OS appearance (`Auto`).
    #[serde(default)]
    pub theme_preference: ThemePreference,

    // --- Dictation modes (config v2+) ---
    #[serde(default)]
    pub modes: Vec<DictationMode>,
    #[serde(default = "default_active_mode_id")]
    pub active_mode_id: String,
    /// Hotkey to cycle `active_mode_id` through `modes`.
    #[serde(default = "default_mode_cycle_hotkey")]
    pub mode_cycle_hotkey: Option<String>,
    /// Hold-to-talk: highlight text, then speak an edit instruction (Phase 5). Empty/unset = disabled.
    #[serde(default)]
    pub voice_edit_hotkey: Option<String>,
    /// Base URL for the community recipe library (HTTPS Worker). No trailing slash (Phase 8).
    #[serde(default = "default_recipe_library_url")]
    pub recipe_library_url: String,

    // --- Phase 9: Pro multi-PC sync ---
    /// Kalam Pro license key (`KALAM-XXXX-...`); used for Bearer auth against the website API (validate + sync).
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default)]
    pub sync_enabled: bool,
    /// ISO timestamp of last successful pull+push (from server `server_time` when available).
    #[serde(default)]
    pub sync_last_at: Option<String>,
    /// Stable per-install id for support; generated when sync is first enabled.
    #[serde(default)]
    pub sync_device_id: Option<String>,
    /// True after local settings/modes/keys change until a successful sync push acknowledges them.
    #[serde(default)]
    pub sync_config_dirty: bool,
    /// Bumped on each successful Settings save; embedded in the synced settings blob for last-write-wins.
    #[serde(default)]
    pub sync_settings_rev: Option<String>,
    /// Last `updated_at` from a merged remote settings blob (LWW on pull).
    #[serde(default)]
    pub sync_last_merged_settings_at: Option<String>,
    /// Last `updated_at` from a merged remote encrypted keys blob.
    #[serde(default)]
    pub sync_last_merged_keys_at: Option<String>,
}

fn default_active_mode_id() -> String {
    "default".to_string()
}

fn default_mode_cycle_hotkey() -> Option<String> {
    Some("Ctrl+Shift+M".to_string())
}

fn default_recipe_library_url() -> String {
    "https://kalam.stream".to_string()
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

/// Command mode: voice-triggered note/task/reminder creation. Provider/model config removed in v6;
/// commands now use the active mode's LLM or the global `default_llm_*` fallback.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandConfig {
    pub enabled: bool,
    pub hotkey: Option<String>,
    // --- Legacy fields: deserialized for migration but never serialized ---
    #[serde(default, skip_serializing)]
    pub provider: Option<String>,
    #[serde(default, skip_serializing)]
    pub api_keys: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing)]
    pub models: std::collections::HashMap<String, String>,
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

/// Preferred overlay layout when dictation is active (Phase 6).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum OverlayActivePreference {
    #[default]
    Mini,
    Full,
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
    #[serde(
        alias = "Line",
        alias = "Symmetric",
        alias = "Heartbeat",
        alias = "Snake",
        alias = "DoubleHelix",
        alias = "Waves",
        alias = "CenterSplit"
    )]
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
    10
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
            overlay_active_preference: OverlayActivePreference::default(),
            overlay_always_visible: false,
            dictation_enabled: true,
            user_email: None,
            marketing_opt_in: false,
            notifications_opt_in: false,
            onboarding_os_name: None,
            onboarding_os_version: None,
            command_config: CommandConfig::default(),
            provider_keys: HashMap::new(),
            default_llm_provider: None,
            default_llm_model: None,
            custom_openai_endpoint: None,
            update_channel: UpdateChannel::Stable,
            update_channel_locked: false,
            sidebar_collapsed: false,
            theme_preference: ThemePreference::default(),
            modes: {
                let ts = chrono::Utc::now().to_rfc3339();
                build_default_modes(&ts)
            },
            active_mode_id: default_active_mode_id(),
            mode_cycle_hotkey: default_mode_cycle_hotkey(),
            voice_edit_hotkey: None,
            recipe_library_url: default_recipe_library_url(),
            license_key: None,
            sync_enabled: false,
            sync_last_at: None,
            sync_device_id: None,
            sync_config_dirty: false,
            sync_settings_rev: None,
            sync_last_merged_settings_at: None,
            sync_last_merged_keys_at: None,
        }
    }
}

/// Built-in modes for a new install or post-migration (Default is non-deletable).
///
/// Canonical source of truth for defaults. The JSON files under `src-tauri/recipes/`
/// mirror these values for reference and community recipe library seeding -- they are
/// NOT loaded at runtime. Keep both in sync when changing built-in mode definitions.
pub fn build_default_modes(now_rfc3339: &str) -> Vec<DictationMode> {
    let ts = now_rfc3339.to_string();
    vec![
        DictationMode {
            id: "default".into(),
            name: "Default".into(),
            icon: Some("ph:microphone".into()),
            accent_color: default_accent_for_mode_id("default"),
            ai_instructions: String::new(),
            voice_model: ModeModelRef::default(),
            language_model: ModeModelRef::default(),
            polish: false,
            context: ModeContextConfig::default(),
            auto_activate_rules: vec![],
            is_builtin: true,
            is_deletable: false,
            created_at: ts.clone(),
            updated_at: ts.clone(),
        },
        DictationMode {
            id: "email".into(),
            name: "Email".into(),
            icon: Some("ph:envelope".into()),
            accent_color: default_accent_for_mode_id("email"),
            ai_instructions: "Format the user's dictation as a professional email body. Add an appropriate greeting and sign-off when suitable. Keep their natural tone; make it clear and professional. Do not add facts they did not say.".into(),
            voice_model: ModeModelRef::default(),
            language_model: ModeModelRef::default(),
            polish: true,
            context: ModeContextConfig {
                enabled: true,
                read_app: true,
                read_clipboard: true,
                read_selection: false,
                include_system_info: true,
            },
            auto_activate_rules: vec![],
            is_builtin: true,
            is_deletable: true,
            created_at: ts.clone(),
            updated_at: ts.clone(),
        },
        DictationMode {
            id: "message".into(),
            name: "Message".into(),
            icon: Some("ph:chat-circle-text".into()),
            accent_color: default_accent_for_mode_id("message"),
            ai_instructions: "Rewrite the user's dictation as a short, casual message suitable for chat or SMS. Be concise and friendly.".into(),
            voice_model: ModeModelRef::default(),
            language_model: ModeModelRef::default(),
            polish: true,
            context: ModeContextConfig::default(),
            auto_activate_rules: vec![],
            is_builtin: true,
            is_deletable: true,
            created_at: ts.clone(),
            updated_at: ts.clone(),
        },
        DictationMode {
            id: "notes".into(),
            name: "Notes".into(),
            icon: Some("ph:note-pencil".into()),
            accent_color: default_accent_for_mode_id("notes"),
            ai_instructions: "Structure the user's dictation as organized notes: use bullet points or short headings where helpful, capture key takeaways, keep their meaning.".into(),
            voice_model: ModeModelRef::default(),
            language_model: ModeModelRef::default(),
            polish: true,
            context: ModeContextConfig::default(),
            auto_activate_rules: vec![],
            is_builtin: true,
            is_deletable: true,
            created_at: ts.clone(),
            updated_at: ts.clone(),
        },
    ]
}

/// Merge global STT settings with a mode's voice model ref (per-mode provider / local model / cloud model id).
pub fn merge_stt_config_for_voice(global: &STTConfig, voice: &ModeModelRef) -> STTConfig {
    let mut cfg = global.clone();
    if voice.is_default() {
        return cfg;
    }
    let p = voice.provider.trim();
    match p.to_lowercase().as_str() {
        "local" => {
            cfg.mode = STTMode::Local;
            if !voice.model_id.trim().is_empty() {
                cfg.local_model = Some(voice.model_id.trim().to_string());
            }
        }
        "groq" | "openai" => {
            cfg.provider = p.to_string();
            cfg.mode = STTMode::Cloud;
            if !voice.model_id.trim().is_empty() {
                cfg.cloud_transcription_model = Some(voice.model_id.trim().to_string());
            } else {
                cfg.cloud_transcription_model = None;
            }
        }
        _ => {}
    }
    cfg
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
    /// Legacy; keys live in `provider_keys` after v3. Still deserialized from old files.
    #[serde(default, skip_serializing)]
    pub api_keys: std::collections::HashMap<String, String>,
    // Legacy field for backwards compatibility; migrated into api_keys on load.
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    pub local_model: Option<String>,
    /// When set (Groq/OpenAI cloud), overrides the provider default transcription model id.
    #[serde(default)]
    pub cloud_transcription_model: Option<String>,
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
            cloud_transcription_model: None,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InjectionMethod {
    Keystrokes,
    Clipboard,
    /// macOS: insert via Accessibility API (`AXSelectedText`); other platforms fall back to clipboard.
    AccessibilityAPI,
    Auto,
}

/// Per-app override for text injection. The foreground process name must **contain**
/// `process_name` (case-insensitive), matching the legacy `force_clipboard_apps` behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppInjectionRule {
    pub process_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub method: InjectionMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keystroke_delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clipboard_threshold: Option<usize>,
}

fn default_app_injection_rules() -> Vec<AppInjectionRule> {
    vec![AppInjectionRule {
        process_name: "notepad.exe".to_string(),
        display_name: Some("Notepad".to_string()),
        method: InjectionMethod::Clipboard,
        keystroke_delay_ms: None,
        clipboard_threshold: None,
    }]
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
    /// Legacy-only: values are merged into `app_injection_rules` when upgrading to config v9.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub force_clipboard_apps: Vec<String>,
    #[serde(default)]
    pub app_injection_rules: Vec<AppInjectionRule>,
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
            force_clipboard_apps: Vec::new(),
            app_injection_rules: default_app_injection_rules(),
        }
    }
}

impl FormattingConfig {
    /// Clamp injection-related fields to the same bounds as the Settings UI (and manual JSON edits).
    pub fn clamp_injection_fields(&mut self) {
        // Cap per-character pause so long dictation stays usable (matches Settings UI).
        const KEYSTROKE_DELAY_MAX_MS: u64 = 100;
        const CLIPBOARD_THRESHOLD_MIN: usize = 1;
        const CLIPBOARD_THRESHOLD_MAX: usize = 100_000;
        const RETRY_ATTEMPTS_MIN: u32 = 1;
        const RETRY_ATTEMPTS_MAX: u32 = 20;
        const RETRY_DELAY_MAX_MS: u64 = 5000;

        self.keystroke_delay_ms = self.keystroke_delay_ms.min(KEYSTROKE_DELAY_MAX_MS);
        self.clipboard_threshold = self
            .clipboard_threshold
            .clamp(CLIPBOARD_THRESHOLD_MIN, CLIPBOARD_THRESHOLD_MAX);
        self.retry_attempts = self
            .retry_attempts
            .clamp(RETRY_ATTEMPTS_MIN, RETRY_ATTEMPTS_MAX);
        self.retry_delay_ms = self.retry_delay_ms.min(RETRY_DELAY_MAX_MS);

        for rule in &mut self.app_injection_rules {
            if let Some(d) = rule.keystroke_delay_ms.as_mut() {
                *d = (*d).min(KEYSTROKE_DELAY_MAX_MS);
            }
            if let Some(t) = rule.clipboard_threshold.as_mut() {
                *t = (*t).clamp(CLIPBOARD_THRESHOLD_MIN, CLIPBOARD_THRESHOLD_MAX);
            }
        }
    }

    /// Clone and apply the first matching per-app rule (substring match on process name).
    pub fn with_applied_app_rules(&self, foreground_process: Option<&str>) -> Self {
        let Some(pname) = foreground_process.filter(|s| !s.trim().is_empty()) else {
            return self.clone();
        };
        let name_lower = pname.to_lowercase();
        for rule in &self.app_injection_rules {
            let needle = rule.process_name.to_lowercase();
            if needle.is_empty() {
                continue;
            }
            if name_lower.contains(&needle) {
                let mut eff = self.clone();
                eff.injection_method = rule.method.clone();
                if let Some(d) = rule.keystroke_delay_ms {
                    eff.keystroke_delay_ms = d;
                }
                if let Some(t) = rule.clipboard_threshold {
                    eff.clipboard_threshold = t;
                }
                return eff;
            }
        }
        self.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingRule {
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
    /// When `false`, `pattern` is matched as literal text (after regex escaping). When the field is
    /// missing in JSON, defaults to `true` so existing configs keep regex semantics.
    #[serde(default = "default_formatting_rule_is_regex")]
    pub is_regex: bool,
}

fn default_formatting_rule_is_regex() -> bool {
    true
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
                pattern: r"(?i)(1password|bitwarden|keepass|lastpass|dashlane|nordpass)"
                    .to_string(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    ProcessName,
    WindowTitle,
    BundleId,
}

/// Auto-switch mode when an app is focused (Phase 7; stored but not enforced in Phase 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoActivateRule {
    pub app_pattern: String,
    pub pattern_type: PatternType,
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
