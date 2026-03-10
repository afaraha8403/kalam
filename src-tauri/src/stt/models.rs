//! Local model download manager: stream from URL, progress events, SHA-256 verification, tar.bz2 extraction.

use serde::Serialize;
use sha2::Digest;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tauri::Emitter;

const SENSEVOICE_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17.tar.bz2";
const SENSEVOICE_SIZE_MB: u64 = 150;
const WHISPER_BASE_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
const WHISPER_BASE_SIZE_MB: u64 = 142;

#[derive(Debug, Clone)]
pub struct ModelManifest {
    pub id: &'static str,
    pub url: &'static str,
    pub expected_sha256: Option<&'static str>,
    pub filename: &'static str,
    pub size_mb: u64,
}

pub fn known_models() -> Vec<ModelManifest> {
    vec![
        ModelManifest {
            id: "sensevoice",
            url: SENSEVOICE_URL,
            expected_sha256: None,
            filename: "sherpa-onnx-sensevoice.tar.bz2",
            size_mb: SENSEVOICE_SIZE_MB,
        },
        ModelManifest {
            id: "whisper_base",
            url: WHISPER_BASE_URL,
            expected_sha256: None,
            filename: "ggml-whisper-base.bin",
            size_mb: WHISPER_BASE_SIZE_MB,
        },
    ]
}

pub fn model_dir() -> anyhow::Result<PathBuf> {
    // Organization + application name both "Kalam" => .../Kalam/Kalam/data/models (do not change to avoid breaking existing config/data paths)
    let dir = directories::ProjectDirs::from("com", "Kalam", "Kalam")
        .ok_or_else(|| anyhow::anyhow!("No project dir"))?
        .data_local_dir()
        .join("models");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Path to the model as used by the sidecar. For .tar.bz2 manifests this is the extracted directory; otherwise the file.
pub fn model_path(model_id: &str, manifest: &ModelManifest) -> anyhow::Result<PathBuf> {
    let dir = model_dir()?;
    if manifest.filename.ends_with(".tar.bz2") {
        Ok(dir.join(model_id))
    } else {
        Ok(dir.join(manifest.filename))
    }
}

/// Path to the downloaded archive or file before extraction (for download destination).
fn download_dest(_model_id: &str, manifest: &ModelManifest) -> anyhow::Result<PathBuf> {
    Ok(model_dir()?.join(manifest.filename))
}

fn is_archive(manifest: &ModelManifest) -> bool {
    manifest.filename.ends_with(".tar.bz2")
}

pub fn is_installed(model_id: &str) -> bool {
    if let Some(m) = known_models().iter().find(|x| x.id == model_id) {
        if let Ok(p) = model_path(model_id, m) {
            if is_archive(m) {
                return p.is_dir() && p.exists();
            }
            return p.exists();
        }
    }
    false
}

/// Directory that contains tokens.txt and encoder/decoder/joiner .onnx (zipformer).
/// May be model_path itself or its single subdir after extraction.
pub fn sherpa_zipformer_model_root(model_path: &Path) -> anyhow::Result<PathBuf> {
    if !model_path.is_dir() {
        return Err(anyhow::anyhow!("Model path is not a directory"));
    }
    let has_tokens = model_path.join("tokens.txt").exists();
    if has_tokens {
        return Ok(model_path.to_path_buf());
    }
    let mut subdirs = std::fs::read_dir(model_path)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect::<Vec<_>>();
    if subdirs.len() == 1 {
        let sub = subdirs.pop().unwrap().path();
        if sub.join("tokens.txt").exists() {
            return Ok(sub);
        }
    }
    Err(anyhow::anyhow!(
        "Could not find tokens.txt in model directory or its subdir"
    ))
}

/// Build sherpa-onnx-offline-websocket-server args for a zipformer/transducer model.
pub fn sherpa_zipformer_server_args(model_root: &Path, port: u16) -> anyhow::Result<Vec<String>> {
    let tokens = model_root.join("tokens.txt");
    if !tokens.exists() {
        return Err(anyhow::anyhow!("tokens.txt not found in {:?}", model_root));
    }
    let find_onnx = |prefix: &str| -> anyhow::Result<PathBuf> {
        for e in std::fs::read_dir(model_root)? {
            let e = e?;
            let p = e.path();
            if p.extension().is_some_and(|x| x == "onnx")
                && p.file_stem()
                    .is_some_and(|s| s.to_string_lossy().starts_with(prefix))
            {
                return Ok(p);
            }
        }
        Err(anyhow::anyhow!(
            "No {}*.onnx found in {:?}",
            prefix,
            model_root
        ))
    };

    // Check if this is a SenseVoice model (has model.onnx or model.int8.onnx)
    if let Ok(model) = find_onnx("model") {
        return Ok(vec![
            format!("--port={}", port),
            format!("--tokens={}", tokens.to_string_lossy()),
            format!("--sense-voice-model={}", model.to_string_lossy()),
        ]);
    }

    let encoder = find_onnx("encoder")?;
    let decoder = find_onnx("decoder")?;
    let joiner = find_onnx("joiner")?;
    let tokens_str = tokens.to_string_lossy().to_string();
    let encoder_str = encoder.to_string_lossy().to_string();
    let decoder_str = decoder.to_string_lossy().to_string();
    let joiner_str = joiner.to_string_lossy().to_string();
    Ok(vec![
        format!("--port={}", port),
        format!("--tokens={}", tokens_str),
        format!("--encoder={}", encoder_str),
        format!("--decoder={}", decoder_str),
        format!("--joiner={}", joiner_str),
    ])
}

#[derive(Clone, Serialize)]
pub struct DownloadProgressPayload {
    pub model_type: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f32>,
}

/// Extract a .tar.bz2 archive to the given directory. Removes the archive on success.
fn extract_tar_bz2(archive_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let dec = bzip2::read::BzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(dec);
    archive.unpack(out_dir)?;
    std::fs::remove_file(archive_path)?;
    Ok(())
}

/// Download a model by id; emit progress via app_handle. Verifies SHA-256 if manifest has it.
/// For .tar.bz2 manifests, extracts to model_dir()/model_id/ and removes the archive.
pub async fn download_model_with_progress(
    model_id: &str,
    app_handle: &tauri::AppHandle,
) -> anyhow::Result<PathBuf> {
    let manifest = known_models()
        .into_iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_id))?;

    let final_path = model_path(model_id, &manifest)?;
    if is_archive(&manifest) {
        if final_path.is_dir() && final_path.exists() {
            log::info!("Model {} already extracted at {:?}", model_id, final_path);
            return Ok(final_path);
        }
    } else if final_path.exists() {
        log::info!("Model {} already at {:?}", model_id, final_path);
        return Ok(final_path);
    }

    let dest = download_dest(model_id, &manifest)?;

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
            total_bytes,
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
            return Err(anyhow::anyhow!(
                "SHA-256 mismatch: expected {}, got {}",
                expected,
                hex
            ));
        }
    }

    if is_archive(&manifest) {
        std::fs::create_dir_all(&final_path)?;
        if let Err(e) = extract_tar_bz2(&dest, &final_path) {
            let _ = std::fs::remove_file(&dest);
            return Err(e);
        }
        log::info!("Downloaded and extracted {} to {:?}", model_id, final_path);
    } else {
        log::info!("Downloaded {} to {:?}", model_id, dest);
    }

    model_path(model_id, &manifest)
}
