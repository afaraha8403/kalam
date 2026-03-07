pub mod privacy;
pub mod settings;

use serde_json;

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
use std::fs;
use std::path::PathBuf;

pub use settings::*;

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

        let mut config = if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            log::debug!("Loading config from {:?}: {}", config_path, contents);
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            log::info!("No existing config found, using defaults");
            AppConfig::default()
        };

        migrate_legacy_languages(&mut config);

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn save(&mut self, config: AppConfig) -> anyhow::Result<()> {
        self.config = config;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                log::info!("Creating parent directory: {:?}", parent);
                match fs::create_dir_all(parent) {
                    Ok(_) => log::info!("Directory created successfully"),
                    Err(e) => {
                        log::error!("Failed to create directory: {}", e);
                        return Err(e.into());
                    }
                }
            }
        }

        log::info!("Serializing config...");
        let json = match serde_json::to_string_pretty(&self.config) {
            Ok(j) => {
                log::info!("Config serialized, JSON length: {}", j.len());
                j
            }
            Err(e) => {
                log::error!("Failed to serialize config: {}", e);
                return Err(e.into());
            }
        };

        log::info!("Writing config to {:?}", self.config_path);
        match fs::write(&self.config_path, json) {
            Ok(_) => {
                log::info!("✓ Config saved successfully to {:?}", self.config_path);
                Ok(())
            }
            Err(e) => {
                log::error!("✗ Failed to write config file: {}", e);
                Err(e.into())
            }
        }
    }

    pub fn get_all(&self) -> AppConfig {
        self.config.clone()
    }

    pub fn get_hotkey(&self) -> String {
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
