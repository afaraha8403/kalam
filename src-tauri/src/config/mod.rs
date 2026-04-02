pub mod privacy;
pub mod settings;

/// Ensure the "default" mode always exists. Re-creates it if someone manually removed it from config.json.
fn ensure_default_mode_exists(config: &mut crate::config::AppConfig) {
    if config.modes.iter().any(|m| m.id == "default") {
        // Force non-deletable even if someone edited the JSON.
        for m in &mut config.modes {
            if m.id == "default" {
                m.is_deletable = false;
            }
        }
        return;
    }
    log::warn!("Default mode missing from config — re-creating it.");
    let ts = chrono::Utc::now().to_rfc3339();
    let defaults = crate::config::build_default_modes(&ts);
    if let Some(default_mode) = defaults.into_iter().find(|m| m.id == "default") {
        config.modes.insert(0, default_mode);
    }
    if config.active_mode_id.trim().is_empty()
        || !config.modes.iter().any(|m| m.id == config.active_mode_id)
    {
        config.active_mode_id = "default".to_string();
    }
}

/// Migrate legacy language/secondary_language into languages if languages is empty.
fn migrate_legacy_languages(config: &mut crate::config::AppConfig) {
    if !config.languages.is_empty() {
        return;
    }
    if let Some(l) = config.language.take() {
        if l != "auto" && !l.is_empty() {
            config.languages.push(l);
        }
    }
    if let Some(s) = config.secondary_language.take() {
        if !s.is_empty() && !config.languages.contains(&s) {
            config.languages.push(s);
        }
    }
    if config.languages.is_empty() {
        config.languages = vec!["en".to_string()];
    }
}

/// Migrate legacy command mode settings to provider-specific maps.
fn migrate_legacy_command_config(config: &mut crate::config::AppConfig) {
    if let Some(provider) = &config.command_config.provider {
        if let Some(api_key) = config.command_config.api_key.take() {
            config
                .command_config
                .api_keys
                .entry(provider.clone())
                .or_insert(api_key);
        }
        if let Some(model) = config.command_config.model.take() {
            config
                .command_config
                .models
                .entry(provider.clone())
                .or_insert(model);
        }
    }
}

/// Migrate legacy STT api_key into provider-specific api_keys map.
fn migrate_legacy_stt_config(config: &mut crate::config::AppConfig) {
    let provider = config.stt_config.provider.clone();
    if let Some(api_key) = config.stt_config.api_key.take() {
        if !provider.trim().is_empty() {
            config
                .stt_config
                .api_keys
                .entry(provider)
                .or_insert(api_key);
        }
    }
}
use std::fs;
use std::path::PathBuf;

pub use settings::*;

/// Current config schema version. Bump when making breaking changes and add a migration.
pub const CURRENT_CONFIG_VERSION: u32 = 8;

/// Run migrations from config's current version to CURRENT_CONFIG_VERSION.
fn migrate_config(mut config: AppConfig) -> AppConfig {
    while config.config_version < CURRENT_CONFIG_VERSION {
        let from = config.config_version;
        config = run_migration(config, from);
        config.config_version = from + 1;
        log::info!("Migrated config from v{} to v{}", from, from + 1);
    }
    config
}

fn run_migration(config: AppConfig, from_version: u32) -> AppConfig {
    match from_version {
        0 => {
            // No version in file (legacy): just ensure version is set; no data change.
            config
        }
        1 => migrate_v1_to_v2(config),
        2 => migrate_v2_to_v3(config),
        3 => migrate_v3_to_v4(config),
        4 => migrate_v4_to_v5(config),
        5 => migrate_v5_to_v6(config),
        6 => migrate_v6_to_v7(config),
        7 => migrate_v7_to_v8(config),
        _ => config,
    }
}

/// Config v8: Rename built-in "voice" mode to "default" (id + name).
fn migrate_v7_to_v8(mut config: AppConfig) -> AppConfig {
    if let Some(m) = config.modes.iter_mut().find(|m| m.id == "voice") {
        m.id = "default".to_string();
        if m.name == "Voice" {
            m.name = "Default".to_string();
        }
        m.is_deletable = false;
    }
    if config.active_mode_id == "voice" {
        config.active_mode_id = "default".to_string();
    }
    config
}

/// Config v7: per-mode `accent_color` for overlay + main UI (empty = derive from mode id).
fn migrate_v6_to_v7(mut config: AppConfig) -> AppConfig {
    for mode in &mut config.modes {
        if mode.accent_color.trim().is_empty() {
            mode.accent_color = crate::config::default_accent_for_mode_id(&mode.id);
        }
    }
    config
}

/// Config v6: Flatten AI config — remove "command" mode, resolve "inherit" refs, populate `default_llm_*`.
fn migrate_v5_to_v6(mut config: AppConfig) -> AppConfig {
    // 1. Promote command_config.provider/model to top-level default_llm_* if not already set.
    if config.default_llm_provider.is_none() {
        if let Some(ref p) = config.command_config.provider {
            let prov = p.trim();
            if !prov.is_empty() {
                let model = config
                    .command_config
                    .models
                    .get(prov)
                    .cloned()
                    .unwrap_or_default();
                config.default_llm_provider = Some(prov.to_string());
                if !model.trim().is_empty() {
                    config.default_llm_model = Some(model);
                }
            }
        }
    }

    // 2. Resolve "inherit" voice_model refs to concrete STT provider/model.
    let global_stt_provider = config.stt_config.provider.clone();
    let global_stt_model = config.stt_config.cloud_transcription_model.clone();
    let global_llm_provider = config.default_llm_provider.clone().unwrap_or_default();
    let global_llm_model = config.default_llm_model.clone().unwrap_or_default();

    for mode in &mut config.modes {
        if mode.voice_model.is_default() {
            mode.voice_model.provider = global_stt_provider.clone();
            if let Some(ref m) = global_stt_model {
                mode.voice_model.model_id = m.clone();
            }
        }
        if mode.language_model.is_default() {
            mode.language_model.provider = global_llm_provider.clone();
            mode.language_model.model_id = global_llm_model.clone();
        }
    }

    // 3. Remove the "command" dictation mode (commands are now a cross-cutting feature, not a mode).
    if config.active_mode_id == "command" {
        config.active_mode_id = "default".to_string();
    }
    config.modes.retain(|m| m.id != "command");

    // 4. Clear legacy command_config fields (they're skip_serializing but clean up in-memory too).
    config.command_config.provider = None;
    config.command_config.api_keys.clear();
    config.command_config.models.clear();
    config.command_config.api_key = None;
    config.command_config.model = None;

    config
}

/// Config v5: Phase 9 sync + optional `license_key` (serde defaults cover new fields when missing in JSON).
fn migrate_v4_to_v5(config: AppConfig) -> AppConfig {
    config
}

/// Config v4: Phase 6 overlay prefs (`overlay_active_preference`, `overlay_always_visible` default via serde).
fn migrate_v3_to_v4(config: AppConfig) -> AppConfig {
    config
}

/// Config v3: unified `provider_keys` from STT + command API key maps.
fn migrate_v2_to_v3(mut config: AppConfig) -> AppConfig {
    for (k, v) in config.stt_config.api_keys.clone() {
        if !k.trim().is_empty() && !v.trim().is_empty() {
            config.provider_keys.entry(k).or_insert(v);
        }
    }
    for (k, v) in config.command_config.api_keys.clone() {
        if !k.trim().is_empty() && !v.trim().is_empty() {
            config.provider_keys.entry(k).or_insert(v);
        }
    }
    config.stt_config.api_keys.clear();
    config.command_config.api_keys.clear();
    config
}

/// Config v2: dictation modes, polish flags, mode-cycle hotkey.
/// Note: the "command" mode is no longer created here (removed in v6), but v5→v6 migration
/// handles cleanup for configs that already went through v1→v2 with a command mode.
fn migrate_v1_to_v2(mut config: AppConfig) -> AppConfig {
    if !config.modes.is_empty() {
        return config;
    }
    let ts = chrono::Utc::now().to_rfc3339();
    config.modes = crate::config::build_default_modes(&ts);
    if config.active_mode_id.trim().is_empty() {
        config.active_mode_id = "default".to_string();
    }
    if config.mode_cycle_hotkey.is_none() {
        config.mode_cycle_hotkey = Some("Ctrl+Shift+M".to_string());
    }
    config
}

/// Extract critical fields from parsed JSON Value so we can preserve them when repair fallback is used.
fn extract_critical_from_value(value: &serde_json::Value) -> (bool, bool) {
    let onboarding = value
        .get("onboarding_complete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let dictation = value
        .get("dictation_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    (onboarding, dictation)
}

/// Best-effort extract onboarding_complete from corrupt JSON via regex (so we don't reset onboarding).
fn extract_onboarding_from_str(contents: &str) -> bool {
    const PATTERN: &str = r#""onboarding_complete"\s*:\s*true"#;
    if let Ok(re) = regex::Regex::new(PATTERN) {
        re.is_match(contents)
    } else {
        false
    }
}

pub fn get_kalam_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|e| anyhow::anyhow!("Could not find home directory: {}", e))?;

    let kalam_dir = PathBuf::from(home).join(".kalam");

    if !kalam_dir.exists() {
        fs::create_dir_all(&kalam_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create directory {:?}: {}", kalam_dir, e))?;
    }

    Ok(kalam_dir)
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl ConfigManager {
    pub fn new() -> anyhow::Result<Self> {
        let kalam_dir = crate::config::get_kalam_dir()?;
        log::info!("Kalam directory: {:?}", kalam_dir);
        let config_path = kalam_dir.join("config.json");
        log::info!("Config will be saved to: {:?}", config_path);

        if !config_path.exists() {
            log::info!("No config file found, using defaults");
            let default = AppConfig::default();
            let mut mgr = Self {
                config_path: config_path.clone(),
                config: default.clone(),
            };
            let _ = mgr.save(default);
            return Ok(mgr);
        }

        let contents = fs::read_to_string(&config_path)?;
        log::debug!("Loading config from {:?}", config_path);

        // When true, persist the loaded config once so the file has the new shape (version/migrations).
        let mut persist_upgraded = false;

        let mut config = match serde_json::from_str::<AppConfig>(&contents) {
            Ok(c) => match c.config_version.cmp(&CURRENT_CONFIG_VERSION) {
                std::cmp::Ordering::Greater => {
                    log::warn!(
                        "Config file version {} is newer than app version {}; using defaults and not overwriting file.",
                        c.config_version,
                        CURRENT_CONFIG_VERSION
                    );
                    AppConfig::default()
                }
                std::cmp::Ordering::Less => {
                    persist_upgraded = true;
                    migrate_config(c)
                }
                std::cmp::Ordering::Equal => {
                    if serde_json::from_str::<serde_json::Value>(&contents)
                        .is_ok_and(|v| v.get("config_version").is_none())
                    {
                        persist_upgraded = true;
                    }
                    c
                }
            },
            Err(e) => {
                log::warn!("Config strict parse failed ({}), attempting auto-fix.", e);
                let mut default = AppConfig::default();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
                    let (onboarding, dictation) = extract_critical_from_value(&value);
                    default.onboarding_complete = onboarding;
                    default.dictation_enabled = dictation;
                    log::info!("Preserved critical flags from JSON Value: onboarding_complete={}, dictation_enabled={}", onboarding, dictation);
                } else {
                    default.onboarding_complete = extract_onboarding_from_str(&contents);
                    log::info!(
                        "Preserved onboarding_complete={} from raw string (corrupt JSON).",
                        default.onboarding_complete
                    );
                }
                let mut mgr = Self {
                    config_path: config_path.clone(),
                    config: default.clone(),
                };
                if mgr.save(default.clone()).is_ok() {
                    log::info!("Repaired config file written.");
                }
                default
            }
        };

        migrate_legacy_languages(&mut config);
        migrate_legacy_stt_config(&mut config);
        migrate_legacy_command_config(&mut config);
        migrate_legacy_hotkeys(&mut config);
        merge_provider_keys_from_nested_maps(&mut config);
        // Safety net: re-create the "default" mode if it was manually removed from config.json.
        ensure_default_mode_exists(&mut config);

        let mut mgr = Self {
            config_path,
            config,
        };
        if persist_upgraded {
            if let Err(e) = mgr.save(mgr.get_all()) {
                log::warn!("Could not persist upgraded config to disk: {}", e);
            } else {
                log::info!("Upgraded config persisted to disk.");
            }
        }

        Ok(mgr)
    }

    pub fn save(&mut self, config: AppConfig) -> anyhow::Result<()> {
        self.config = config;
        self.config.config_version = CURRENT_CONFIG_VERSION;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                log::info!("Creating parent directory: {:?}", parent);
                fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
            }
        }

        log::info!("Serializing config...");
        let json = serde_json::to_string_pretty(&self.config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
        log::info!("Config serialized, JSON length: {}", json.len());

        let tmp_path = self.config_path.with_extension("json.tmp");
        log::info!("Writing config to temp {:?}", tmp_path);
        fs::write(&tmp_path, &json)
            .map_err(|e| anyhow::anyhow!("Failed to write temp config: {}", e))?;
        fs::rename(&tmp_path, &self.config_path)
            .map_err(|e| anyhow::anyhow!("Failed to rename temp to config: {}", e))?;
        log::info!("Config saved successfully to {:?}", self.config_path);
        Ok(())
    }

    pub fn get_all(&self) -> AppConfig {
        self.config.clone()
    }

    pub fn get_hotkey(&self) -> Option<String> {
        self.config.hotkey.clone()
    }

    pub fn get_stt_config(&self) -> STTConfig {
        self.config.stt_config.clone()
    }

    pub fn get_snippets(&self) -> Vec<Snippet> {
        self.config.snippets.clone()
    }

    pub fn add_snippet(&mut self, trigger: String, expansion: String) -> anyhow::Result<()> {
        // Remove existing snippet with same trigger
        self.config.snippets.retain(|s| s.trigger != trigger);
        self.config.snippets.push(Snippet { trigger, expansion });
        self.save(self.config.clone())
    }

    pub fn remove_snippet(&mut self, trigger: &str) -> anyhow::Result<()> {
        self.config.snippets.retain(|s| s.trigger != trigger);
        self.save(self.config.clone())
    }
}

pub use settings::Snippet;

/// Mark config as needing a multi-PC sync push (used when modes/keys change outside Settings save).
pub fn bump_sync_config_dirty(config: &mut AppConfig) {
    config.sync_config_dirty = true;
    config.sync_settings_rev = Some(chrono::Utc::now().to_rfc3339());
}

/// Copy keys from nested legacy maps into `provider_keys` (idempotent; runs on every load for old configs).
pub fn merge_provider_keys_from_nested_maps(config: &mut AppConfig) {
    for (k, v) in config.stt_config.api_keys.clone() {
        if !k.trim().is_empty() && !v.trim().is_empty() {
            config.provider_keys.entry(k).or_insert(v);
        }
    }
    // command_config.api_keys is legacy (skip_serializing) but may exist in pre-v6 JSON on disk.
    for (k, v) in config.command_config.api_keys.clone() {
        if !k.trim().is_empty() && !v.trim().is_empty() {
            config.provider_keys.entry(k).or_insert(v);
        }
    }
}

fn migrate_legacy_hotkeys(config: &mut AppConfig) {
    if let Some(mode) = &config.recording_mode {
        if matches!(mode, crate::config::RecordingMode::Toggle) {
            // If it was toggle mode, move the hotkey to toggle_dictation_hotkey
            if config.toggle_dictation_hotkey.is_none() {
                config.toggle_dictation_hotkey = config.hotkey.clone();
                config.hotkey = None;
            }
        }
        config.recording_mode = None;
    }
}
