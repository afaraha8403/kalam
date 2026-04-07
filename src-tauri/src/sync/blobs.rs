//! Subset of `AppConfig` that Phase 9 syncs (no device-specific or non-synced fields).

use crate::config::{
    AppConfig, FormattingConfig, PrivacyConfig, RecordingMode, STTConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettingsBlob {
    pub updated_at: String,
    pub active_mode_id: String,
    pub formatting: FormattingConfig,
    pub recording_mode: Option<RecordingMode>,
    pub hotkey: Option<String>,
    pub toggle_dictation_hotkey: Option<String>,
    pub language_toggle_hotkey: Option<String>,
    pub mode_cycle_hotkey: Option<String>,
    pub voice_edit_hotkey: Option<String>,
    pub min_hold_ms: u64,
    pub languages: Vec<String>,
    pub stt_config: STTConfig,
    /// v6+: only `enabled` and `hotkey` are meaningful; legacy fields deserialized for compat.
    pub command_enabled: bool,
    pub command_hotkey: Option<String>,
    pub default_llm_provider: Option<String>,
    pub default_llm_model: Option<String>,
    pub privacy: PrivacyConfig,
    pub dictation_enabled: bool,
}

fn stt_for_sync(c: &STTConfig) -> STTConfig {
    let mut s = c.clone();
    s.api_keys.clear();
    s.api_key = None;
    s
}

impl SyncSettingsBlob {
    pub fn from_config(cfg: &AppConfig, updated_at: &str) -> Self {
        Self {
            updated_at: updated_at.to_string(),
            active_mode_id: cfg.active_mode_id.clone(),
            formatting: cfg.formatting.clone(),
            recording_mode: cfg.recording_mode.clone(),
            hotkey: cfg.hotkey.clone(),
            toggle_dictation_hotkey: cfg.toggle_dictation_hotkey.clone(),
            language_toggle_hotkey: cfg.language_toggle_hotkey.clone(),
            mode_cycle_hotkey: cfg.mode_cycle_hotkey.clone(),
            voice_edit_hotkey: cfg.voice_edit_hotkey.clone(),
            min_hold_ms: cfg.min_hold_ms,
            languages: cfg.languages.clone(),
            stt_config: stt_for_sync(&cfg.stt_config),
            command_enabled: cfg.command_config.enabled,
            command_hotkey: cfg.command_config.hotkey.clone(),
            default_llm_provider: cfg.default_llm_provider.clone(),
            default_llm_model: cfg.default_llm_model.clone(),
            privacy: cfg.privacy.clone(),
            dictation_enabled: cfg.dictation_enabled,
        }
    }

    pub fn apply_to(&self, cfg: &mut AppConfig) {
        cfg.active_mode_id = self.active_mode_id.clone();
        cfg.formatting = self.formatting.clone();
        cfg.recording_mode = self.recording_mode.clone();
        cfg.hotkey = self.hotkey.clone();
        cfg.toggle_dictation_hotkey = self.toggle_dictation_hotkey.clone();
        cfg.language_toggle_hotkey = self.language_toggle_hotkey.clone();
        cfg.mode_cycle_hotkey = self.mode_cycle_hotkey.clone();
        cfg.voice_edit_hotkey = self.voice_edit_hotkey.clone();
        cfg.min_hold_ms = self.min_hold_ms;
        cfg.languages = self.languages.clone();
        cfg.stt_config = self.stt_config.clone();
        cfg.command_config.enabled = self.command_enabled;
        cfg.command_config.hotkey = self.command_hotkey.clone();
        cfg.default_llm_provider = self.default_llm_provider.clone();
        cfg.default_llm_model = self.default_llm_model.clone();
        cfg.privacy = self.privacy.clone();
        cfg.dictation_enabled = self.dictation_enabled;
    }
}
