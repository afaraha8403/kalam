//! Local STT: SenseVoice via Sherpa-ONNX WebSocket; Whisper Base via whisper.cpp server HTTP.

use futures_util::{SinkExt, StreamExt};
use std::io::Write;
use std::path::PathBuf;
use tauri::Manager;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use super::provider::STTProvider;
use super::TranscriptionResult;
use crate::AppState;

pub const SIDECAR_NAME: &str = "sherpa-onnx";

const WS_PORT: u16 = 10080;

pub struct SenseVoiceProvider {
    app_handle: tauri::AppHandle,
    model_path: PathBuf,
    model_id: String,
}

impl SenseVoiceProvider {
    pub fn new(app_handle: tauri::AppHandle, model_path: PathBuf, model_id: String) -> Self {
        Self {
            app_handle,
            model_path,
            model_id,
        }
    }

    fn ensure_server_running(&self) -> anyhow::Result<()> {
        let state = self.app_handle.state::<AppState>();
        let model_id = self.model_id.clone();

        tauri::async_runtime::block_on(async move {
            let status = state.local_model_manager.get_status(&model_id).await;
            if status != crate::stt::lifecycle::ModelStatus::Running {
                log::info!(
                    "Local STT server for {} not running, starting it...",
                    model_id
                );
                state.local_model_manager.start_model(&model_id).await?;
                // Wait a bit for the server to start
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn display_name(&self) -> &str {
        match self.model_id.as_str() {
            "whisper_base" => "Whisper Base (Local)",
            "sensevoice" => "SenseVoice (Local)",
            _ => "Local STT",
        }
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

        let text = if self.model_id == "whisper_base" {
            transcribe_via_whisper_http(audio, sample_rate)?
        } else {
            transcribe_via_websocket(audio, sample_rate)?
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
        self.display_name()
    }
}

/// Protocol: connect to ws://127.0.0.1:port, send binary [sample_rate LE 4][samples_byte_len LE 4][float32 samples]
/// (chunked like the official Python client to avoid server buffer issues), recv text, send "Done".
const CHUNK_SIZE: usize = 10240;

fn transcribe_via_websocket(audio: &[f32], sample_rate: u32) -> anyhow::Result<String> {
    let mut payload = Vec::with_capacity(8 + audio.len() * 4);
    payload.extend_from_slice(&sample_rate.to_le_bytes());
    let samples_bytes = (audio.len() * 4) as u32;
    payload.extend_from_slice(&samples_bytes.to_le_bytes());
    for &s in audio {
        payload.extend_from_slice(&s.to_le_bytes());
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let url = format!("ws://127.0.0.1:{}/", WS_PORT);
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {}", e))?;
        let (mut write, mut read) = ws_stream.split();

        // Send payload in chunks to match sherpa-onnx Python client; single large frame can trigger server crash (e.g. STATUS_STACK_BUFFER_OVERRUN on Windows).
        let mut offset = 0;
        while offset < payload.len() {
            let end = (offset + CHUNK_SIZE).min(payload.len());
            write
                .send(Message::Binary(payload[offset..end].to_vec()))
                .await
                .map_err(|e| anyhow::anyhow!("WebSocket send failed: {}", e))?;
            offset = end;
        }

        let result_msg = read
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No response from server"))??;
        let text = match result_msg {
            Message::Text(s) => {
                // The server returns a JSON string like: {"lang": "", "emotion": "", "event": "", "text": " TESTING TESTING ONE TO THREE TESTING", ...}
                // We need to parse it and extract the "text" field.
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                    if let Some(t) = json.get("text").and_then(|v| v.as_str()) {
                        t.to_string()
                    } else {
                        s
                    }
                } else {
                    s
                }
            }
            Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
            _ => return Err(anyhow::anyhow!("Unexpected message type")),
        };

        write
            .send(Message::Text("Done".to_string()))
            .await
            .map_err(|e| anyhow::anyhow!("WebSocket send Done failed: {}", e))?;

        Ok::<String, anyhow::Error>(text)
    })
}

/// Whisper.cpp server: POST multipart to /inference with WAV file, response_format=json; response is {"text": "..."}.
const WHISPER_HTTP_PORT: u16 = 10080;

fn transcribe_via_whisper_http(audio: &[f32], sample_rate: u32) -> anyhow::Result<String> {
    let samples_i16: Vec<i16> = audio
        .iter()
        .map(|&s| {
            let v = (s * 32767.0_f32).round();
            v.clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect();
    let wav = create_wav(&samples_i16, sample_rate, 1)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let url = format!("http://127.0.0.1:{}/inference", WHISPER_HTTP_PORT);
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("response_format", "json");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        let resp = client.post(&url).multipart(form).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("Whisper server error {}: {}", status, body));
        }
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("Whisper response not JSON: {}", e))?;
        if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
            return Err(anyhow::anyhow!("Whisper server: {}", err));
        }
        let text = json
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok::<String, anyhow::Error>(text)
    })
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
