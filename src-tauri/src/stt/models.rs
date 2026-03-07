//! Local model download manager: stream from URL, progress events, SHA-256 verification.

use serde::Serialize;
use sha2::Digest;
use std::path::PathBuf;
use tauri::Emitter;

const SENSEVOICE_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2";
const SENSEVOICE_SIZE_MB: u64 = 200;

#[derive(Debug, Clone)]
pub struct ModelManifest {
    pub id: &'static str,
    pub url: &'static str,
    pub expected_sha256: Option<&'static str>,
    pub filename: &'static str,
    pub size_mb: u64,
}

pub fn known_models() -> Vec<ModelManifest> {
    vec![ModelManifest {
        id: "sensevoice",
        url: SENSEVOICE_URL,
        expected_sha256: None,
        filename: "sherpa-onnx-sensevoice.tar.bz2",
        size_mb: SENSEVOICE_SIZE_MB,
    }]
}

pub fn model_dir() -> anyhow::Result<PathBuf> {
    let dir = directories::ProjectDirs::from("com", "Kalam", "Kalam Voice")
        .ok_or_else(|| anyhow::anyhow!("No project dir"))?
        .data_local_dir()
        .join("models");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn model_path(_model_id: &str, manifest: &ModelManifest) -> anyhow::Result<PathBuf> {
    Ok(model_dir()?.join(manifest.filename))
}

pub fn is_installed(model_id: &str) -> bool {
    if let Some(m) = known_models().iter().find(|x| x.id == model_id) {
        if let Ok(p) = model_path(model_id, m) {
            return p.exists();
        }
    }
    false
}

#[derive(Clone, Serialize)]
pub struct DownloadProgressPayload {
    pub model_type: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f32>,
}

/// Download a model by id; emit progress via app_handle. Verifies SHA-256 if manifest has it.
pub async fn download_model_with_progress(
    model_id: &str,
    app_handle: &tauri::AppHandle,
) -> anyhow::Result<PathBuf> {
    let manifest = known_models()
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_id))?;

    let dest = model_path(model_id, &manifest)?;
    if dest.exists() {
        log::info!("Model {} already at {:?}", model_id, dest);
        return Ok(dest);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()?;
    let response = client.get(manifest.url).send().await?;
    let total_bytes = response.content_length();
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed: {}", response.status()));
    }

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = tokio::fs::File::create(&dest).await?;

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let len = chunk.len() as u64;
        downloaded += len;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        let percent = total_bytes.map(|t| (downloaded as f32 / t as f32) * 100.0);
        let payload = DownloadProgressPayload {
            model_type: model_id.to_string(),
            downloaded_bytes: downloaded,
            total_bytes: total_bytes,
            percent,
        };
        let _ = app_handle.emit("model-download-progress", &payload);
    }

    file.sync_all().await?;
    drop(file);

    if let Some(expected) = manifest.expected_sha256 {
        let data = tokio::fs::read(&dest).await?;
        let digest = sha2::Sha256::digest(&data);
        let hex = format!("{:x}", digest);
        if hex != expected {
            let _ = tokio::fs::remove_file(&dest).await;
            return Err(anyhow::anyhow!("SHA-256 mismatch: expected {}, got {}", expected, hex));
        }
    }

    log::info!("Downloaded {} to {:?}", model_id, dest);
    Ok(dest)
}
