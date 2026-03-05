#![allow(dead_code)]

use super::TranscriptionResult;
use crate::config::STTConfig;
use std::pin::Pin;
use std::future::Future;

pub struct STTProviderFactory;

impl STTProviderFactory {
    pub fn create(config: &STTConfig) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn STTProvider>>> + Send + '_>> {
        Box::pin(async move {
            match config.mode {
                crate::config::STTMode::Cloud => {
                    match config.provider.as_str() {
                        "groq" => {
                            let api_key = config.api_key.as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Groq API key not set"))?;
                            Ok(Box::new(super::groq::GroqProvider::new(api_key.clone())?) as Box<dyn STTProvider>)
                        }
                        _ => Err(anyhow::anyhow!("Unknown provider: {}", config.provider)),
                    }
                }
                crate::config::STTMode::Local => {
                    Err(anyhow::anyhow!("Local STT not yet implemented"))
                }
                crate::config::STTMode::Hybrid | crate::config::STTMode::Auto => {
                    let mut cloud_config = config.clone();
                    cloud_config.mode = crate::config::STTMode::Cloud;
                    Self::create(&cloud_config).await
                }
            }
        })
    }
}

pub trait STTProvider: Send + Sync {
    fn transcribe_blocking(&self, audio: &[f32]) -> anyhow::Result<TranscriptionResult>;
    fn requires_internet(&self) -> bool;
    fn name(&self) -> &str;
}
