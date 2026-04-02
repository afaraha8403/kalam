#![allow(dead_code)]

use super::TranscriptionResult;
use crate::config::{STTConfig, STTMode};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Future returned by STTProviderFactory::create.
pub type CreateProviderFuture<'a> =
    Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn STTProvider>>> + Send + 'a>>;

pub struct STTProviderFactory;

fn selected_api_key(
    config: &STTConfig,
    provider_keys: Option<&HashMap<String, String>>,
) -> Option<String> {
    let prov = config.provider.trim();
    if let Some(pk) = provider_keys {
        if let Some(k) = pk.get(prov) {
            let k = k.trim();
            if !k.is_empty() {
                return Some(k.to_string());
            }
        }
    }
    config
        .api_keys
        .get(&config.provider)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            config
                .api_key
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

/// Per-request ceiling for cloud STT HTTP calls; floored so we do not regress below the old 30s failure window.
fn cloud_transcription_http_timeout(config: &STTConfig) -> std::time::Duration {
    const FLOOR_SECS: u64 = 45;
    let secs = config
        .transcription_timeout
        .timeout_max_seconds
        .max(FLOOR_SECS);
    std::time::Duration::from_secs(secs)
}

/// Create a provider on the current thread. Used so that reqwest::blocking::Client
/// is never created or dropped on a tokio worker (avoids "Cannot drop a runtime" panic).
/// For Local mode, pass Some(app_handle); for Cloud/Groq pass None.
pub fn create_provider_sync(
    config: &STTConfig,
    provider_keys: Option<&HashMap<String, String>>,
    app_handle: Option<&tauri::AppHandle>,
) -> anyhow::Result<Box<dyn STTProvider>> {
    match config.mode {
        STTMode::Cloud | STTMode::Hybrid | STTMode::Auto => match config.provider.as_str() {
            "groq" => {
                let api_key = selected_api_key(config, provider_keys)
                    .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
                let t = cloud_transcription_http_timeout(config);
                Ok(Box::new(super::groq::GroqProvider::new(
                    api_key.clone(),
                    t,
                    config.cloud_transcription_model.clone(),
                )?) as Box<dyn STTProvider>)
            }
            "openai" => {
                let api_key = selected_api_key(config, provider_keys)
                    .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
                let t = cloud_transcription_http_timeout(config);
                Ok(Box::new(super::openai::OpenAIProvider::new(
                    api_key.clone(),
                    t,
                    config.cloud_transcription_model.clone(),
                )?) as Box<dyn STTProvider>)
            }
            _ => Err(anyhow::anyhow!("Unknown provider: {}", config.provider)),
        },
        STTMode::Local => {
            let handle = app_handle
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("App handle required for Local STT"))?;
            let local_model_id = config
                .local_model
                .clone()
                .unwrap_or_else(|| "sensevoice".to_string());
            let manifest = super::models::known_models()
                .into_iter()
                .find(|m| m.id == local_model_id.as_str())
                .ok_or_else(|| anyhow::anyhow!("Local model manifest not found"))?;
            let model_path = super::models::model_path(&local_model_id, &manifest)?;
            if !super::models::is_installed(&local_model_id) {
                log::warn!(
                    "Local model check failed: model_id={}, model_path={:?}, exists={}",
                    local_model_id,
                    model_path,
                    model_path.exists()
                );
                return Err(anyhow::anyhow!(
                    "Selected local model is not installed. Download it in Settings."
                ));
            }
            Ok(Box::new(super::sensevoice::SenseVoiceProvider::new(
                handle,
                model_path,
                local_model_id,
            )) as Box<dyn STTProvider>)
        }
    }
}

impl STTProviderFactory {
    /// Create provider for the given config. For Local mode, app_handle is required for the sidecar.
    pub fn create(
        config: &STTConfig,
        app_handle: Option<tauri::AppHandle>,
    ) -> CreateProviderFuture<'_> {
        let config = config.clone();
        let app_handle = app_handle.clone();
        Box::pin(async move {
            match config.mode {
                crate::config::STTMode::Cloud => match config.provider.as_str() {
                    "groq" => {
                        let api_key = selected_api_key(&config, None)
                            .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
                        let t = cloud_transcription_http_timeout(&config);
                        Ok(Box::new(super::groq::GroqProvider::new(
                            api_key.clone(),
                            t,
                            config.cloud_transcription_model.clone(),
                        )?) as Box<dyn STTProvider>)
                    }
                    "openai" => {
                        let api_key = selected_api_key(&config, None)
                            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
                        let t = cloud_transcription_http_timeout(&config);
                        Ok(Box::new(super::openai::OpenAIProvider::new(
                            api_key.clone(),
                            t,
                            config.cloud_transcription_model.clone(),
                        )?) as Box<dyn STTProvider>)
                    }
                    _ => Err(anyhow::anyhow!("Unknown provider: {}", config.provider)),
                },
                crate::config::STTMode::Local => {
                    let handle = app_handle
                        .ok_or_else(|| anyhow::anyhow!("App handle required for Local STT"))?;
                    let local_model_id = config
                        .local_model
                        .clone()
                        .unwrap_or_else(|| "sensevoice".to_string());
                    let manifest = super::models::known_models()
                        .into_iter()
                        .find(|m| m.id == local_model_id.as_str())
                        .ok_or_else(|| anyhow::anyhow!("Local model manifest not found"))?;
                    let model_path = super::models::model_path(&local_model_id, &manifest)?;
                    if !super::models::is_installed(&local_model_id) {
                        log::warn!(
                            "Local model check failed: model_id={}, model_path={:?}, exists={}",
                            local_model_id,
                            model_path,
                            model_path.exists()
                        );
                        return Err(anyhow::anyhow!(
                            "Selected local model is not installed. Download it in Settings."
                        ));
                    }
                    Ok(Box::new(super::sensevoice::SenseVoiceProvider::new(
                        handle,
                        model_path,
                        local_model_id,
                    )) as Box<dyn STTProvider>)
                }
                crate::config::STTMode::Hybrid | crate::config::STTMode::Auto => {
                    let mut cloud_config = config.clone();
                    cloud_config.mode = crate::config::STTMode::Cloud;
                    Self::create(&cloud_config, app_handle).await
                }
            }
        })
    }
}

pub trait STTProvider: Send + Sync {
    /// Transcribe audio. Optional prompt (e.g. previous chunk text) helps maintain context.
    /// language_hint: if Some (e.g. "en"), use for API when supported.
    fn transcribe_blocking(
        &self,
        audio: &[f32],
        sample_rate: u32,
        prompt: Option<&str>,
        language_hint: Option<&str>,
    ) -> anyhow::Result<TranscriptionResult>;
    fn requires_internet(&self) -> bool;
    fn name(&self) -> &str;
}

#[cfg(test)]
mod local_provider_tests {
    use super::create_provider_sync;
    use crate::config::{STTConfig, STTMode};

    #[test]
    fn local_mode_errors_without_app_handle() {
        let mut cfg = STTConfig::default();
        cfg.mode = STTMode::Local;
        cfg.local_model = Some("sensevoice".to_string());
        let err = match create_provider_sync(&cfg, None, None) {
            Err(e) => e,
            Ok(_) => panic!("expected error without AppHandle"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("App handle"),
            "expected App handle error, got: {}",
            msg
        );
    }
}
