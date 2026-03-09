//! SenseVoice local STT via Sherpa-ONNX sidecar.

use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

use super::provider::STTProvider;
use super::TranscriptionResult;
use crate::AppState;

pub const SIDECAR_NAME: &str = "sherpa-onnx";

pub struct SenseVoiceProvider {
    app_handle: tauri::AppHandle,
    model_path: PathBuf,
    client: reqwest::blocking::Client,
}

impl SenseVoiceProvider {
    pub fn new(app_handle: tauri::AppHandle, model_path: PathBuf) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            app_handle,
            model_path,
            client,
        }
    }

    fn ensure_server_running(&self) -> anyhow::Result<()> {
        let state = self.app_handle.state::<AppState>();

        tauri::async_runtime::block_on(async {
            let status = state.local_model_manager.get_status("sensevoice").await;
            if status != crate::stt::lifecycle::ModelStatus::Running {
                log::info!("SenseVoice server not running, starting it...");
                state.local_model_manager.start_model("sensevoice").await?;
                // Wait a bit for the server to start
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }
}

impl STTProvider for SenseVoiceProvider {
    fn transcribe_blocking(
        &self,
        audio: &[f32],
        sample_rate: u32,
        _prompt: Option<&str>,
        _language_hint: Option<&str>,
    ) -> anyhow::Result<TranscriptionResult> {
        self.ensure_server_running()?;

        let samples_i16: Vec<i16> = audio
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        let wav = create_wav(&samples_i16, sample_rate, 1)?;

        let text = match self
            .client
            .post("http://localhost:10080/transcribe")
            .body(wav.clone())
            .send()
        {
            Ok(response) if response.status().is_success() => response.text()?,
            Ok(response) => {
                log::warn!(
                    "Local server returned {}. Falling back to one-shot sidecar.",
                    response.status()
                );
                run_sidecar_sync(self.app_handle.clone(), &self.model_path, &wav)?
            }
            Err(e) => {
                log::warn!(
                    "Local server request failed: {}. Falling back to one-shot sidecar.",
                    e
                );
                run_sidecar_sync(self.app_handle.clone(), &self.model_path, &wav)?
            }
        };

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            confidence: 0.9,
            language: "unknown".to_string(),
        })
    }

    fn requires_internet(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "SenseVoice (Local)"
    }
}

fn run_sidecar_sync(
    app_handle: tauri::AppHandle,
    model_path: &std::path::Path,
    wav_bytes: &[u8],
) -> anyhow::Result<String> {
    let temp_dir = std::env::temp_dir();
    let input_path = temp_dir.join(format!("kalam_sensevoice_{}.wav", std::process::id()));
    std::fs::write(&input_path, wav_bytes)?;
    let input_path_for_sidecar = input_path.clone();

    let output = tauri::async_runtime::block_on(async move {
        app_handle
            .shell()
            .sidecar(SIDECAR_NAME)
            .map_err(|e| anyhow::anyhow!("Sidecar not found: {}", e))?
            .args([
                "--model",
                model_path.to_str().unwrap_or(""),
                "--input",
                input_path_for_sidecar.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Sidecar execution failed: {}", e))
    })?;

    let _ = std::fs::remove_file(&input_path);
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("SenseVoice sidecar stderr: {}", stderr);
    }
    Ok(stdout)
}

fn create_wav(samples: &[i16], sample_rate: u32, channels: u16) -> anyhow::Result<Vec<u8>> {
    let data_len = samples.len() * 2;
    let file_len = 36 + data_len;
    let mut wav = Vec::with_capacity(44 + data_len);
    wav.write_all(b"RIFF")?;
    wav.write_all(&(file_len as u32).to_le_bytes())?;
    wav.write_all(b"WAVE")?;
    wav.write_all(b"fmt ")?;
    wav.write_all(&16u32.to_le_bytes())?;
    wav.write_all(&1u16.to_le_bytes())?;
    wav.write_all(&channels.to_le_bytes())?;
    wav.write_all(&sample_rate.to_le_bytes())?;
    wav.write_all(&(sample_rate * channels as u32 * 2).to_le_bytes())?;
    wav.write_all(&(channels * 2).to_le_bytes())?;
    wav.write_all(&16u16.to_le_bytes())?;
    wav.write_all(b"data")?;
    wav.write_all(&(data_len as u32).to_le_bytes())?;
    for &sample in samples {
        wav.write_all(&sample.to_le_bytes())?;
    }
    Ok(wav)
}
