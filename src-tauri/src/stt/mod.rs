#![allow(dead_code)]

pub mod groq;
pub mod provider;

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
}

pub struct STTManager;

impl STTManager {
    pub fn new() -> Self {
        Self
    }
}

pub async fn download_model(model_type: &str) -> anyhow::Result<()> {
    log::info!("Downloading model: {}", model_type);
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok(())
}

pub async fn get_model_status() -> anyhow::Result<serde_json::Value> {
    use serde_json::json;
    
    Ok(json!({
        "sensevoice": {
            "installed": false,
            "size_mb": 200,
            "download_progress": null
        },
        "whisper_base": {
            "installed": false,
            "size_mb": 142,
            "download_progress": null
        }
    }))
}

pub async fn validate_api_key(provider: &str, api_key: &str) -> anyhow::Result<bool> {
    match provider {
        "groq" => groq::validate_key(api_key).await,
        _ => Ok(false),
    }
}
