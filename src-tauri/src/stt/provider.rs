#![allow(dead_code)]

use super::TranscriptionResult;
use crate::config::{STTConfig, STTMode};
use std::future::Future;
use std::pin::Pin;

/// Future returned by STTProviderFactory::create.
pub type CreateProviderFuture<'a> =
    Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn STTProvider>>> + Send + 'a>>;

pub struct STTProviderFactory;

/// Create a Cloud/Groq provider on the current thread. Used so that reqwest::blocking::Client
/// is never created or dropped on a tokio worker (avoids "Cannot drop a runtime" panic).
/// Handles Cloud and Hybrid/Auto (when no sensitive app matched, use Cloud/Groq).
pub fn create_provider_sync(config: &STTConfig) -> anyhow::Result<Box<dyn STTProvider>> {
    match config.mode {
        STTMode::Cloud | STTMode::Hybrid | STTMode::Auto => match config.provider.as_str() {
            "groq" => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
                Ok(Box::new(super::groq::GroqProvider::new(api_key.clone())?)
                    as Box<dyn STTProvider>)
            }
            "openai" => {
                let api_key = config
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
                Ok(
                    Box::new(super::openai::OpenAIProvider::new(api_key.clone())?)
                        as Box<dyn STTProvider>,
                )
            }
            _ => Err(anyhow::anyhow!("Unknown provider: {}", config.provider)),
        },
        STTMode::Local => Err(anyhow::anyhow!(
            "create_provider_sync only supports Cloud/Groq (Local needs app_handle)"
        )),
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
                        let api_key = config
                            .api_key
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
                        Ok(Box::new(super::groq::GroqProvider::new(api_key.clone())?)
                            as Box<dyn STTProvider>)
                    }
                    "openai" => {
                        let api_key = config
                            .api_key
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
                        Ok(
                            Box::new(super::openai::OpenAIProvider::new(api_key.clone())?)
                                as Box<dyn STTProvider>,
                        )
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
                    if !model_path.exists() {
                        return Err(anyhow::anyhow!(
                            "Selected local model is not installed. Download it in Settings."
                        ));
                    }
                    Ok(Box::new(super::sensevoice::SenseVoiceProvider::new(
                        handle, model_path,
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
